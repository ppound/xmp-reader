//! IContextMenu shell extension that adds "Copy with sidecar" and
//! "Move with sidecar" to the Explorer right-click menu for supported
//! image file types.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Ole::*;
use windows::Win32::System::Registry::HKEY;
use windows::Win32::UI::Shell::Common;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::sidecar;

// Menu command IDs (offsets from idCmdFirst).
const CMD_COPY: u32 = 0;
const CMD_MOVE: u32 = 1;
const CMD_COUNT: u32 = 2;

const MENU_COPY_TEXT: &str = "Copy with sidecar";
const MENU_MOVE_TEXT: &str = "Move with sidecar";

/// An image file with an optional sidecar.
struct FileEntry {
    image: PathBuf,
    sidecar: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Context menu handler -- one instance per right-click invocation
// ---------------------------------------------------------------------------

#[implement(IShellExtInit, IContextMenu)]
pub struct ContextMenuHandler {
    state: Mutex<MenuState>,
}

struct MenuState {
    /// All selected image files, each with an optional sidecar.
    entries: Vec<FileEntry>,
    /// Whether at least one selected file has a sidecar.
    has_any_sidecar: bool,
    /// The first command ID we were assigned by Explorer.
    id_cmd_first: u32,
}

impl ContextMenuHandler {
    fn new() -> Self {
        Self {
            state: Mutex::new(MenuState {
                entries: Vec::new(),
                has_any_sidecar: false,
                id_cmd_first: 0,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// IShellExtInit -- Explorer calls this to hand us the selected file(s)
// ---------------------------------------------------------------------------

impl IShellExtInit_Impl for ContextMenuHandler_Impl {
    fn Initialize(
        &self,
        _pidlfolder: *const Common::ITEMIDLIST,
        pdtobj: Option<&IDataObject>,
        _hkeyprogid: HKEY,
    ) -> Result<()> {
        let data_obj = pdtobj.ok_or_else(|| Error::from(E_INVALIDARG))?;

        // Ask the data object for the list of selected files via CF_HDROP.
        let fmt = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: core::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };

        let medium = unsafe { data_obj.GetData(&fmt)? };

        // medium.u.hGlobal contains an HDROP.
        let hdrop = HDROP(unsafe { medium.u.hGlobal.0 } as *mut _);

        let count = unsafe { DragQueryFileW(hdrop, 0xFFFFFFFF, None) };
        if count == 0 {
            unsafe { ReleaseStgMedium(&medium as *const _ as *mut _) };
            return Err(Error::from(E_FAIL));
        }

        // Extract all selected file paths and find their sidecars.
        let mut entries = Vec::new();
        let mut has_any_sidecar = false;
        for i in 0..count {
            let needed = unsafe { DragQueryFileW(hdrop, i, None) } + 1;
            let mut buf = vec![0u16; needed as usize];
            unsafe { DragQueryFileW(hdrop, i, Some(&mut buf)) };

            let path_str = String::from_utf16_lossy(&buf[..buf.len() - 1]);
            let image_path = PathBuf::from(&path_str);
            let sidecar = sidecar::find_sidecar(&image_path);
            if sidecar.is_some() {
                has_any_sidecar = true;
            }
            entries.push(FileEntry {
                image: image_path,
                sidecar,
            });
        }

        unsafe { ReleaseStgMedium(&medium as *const _ as *mut _) };

        let mut state = self.state.lock().unwrap();
        state.entries = entries;
        state.has_any_sidecar = has_any_sidecar;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// IContextMenu -- add menu items and handle invocation
// ---------------------------------------------------------------------------

impl IContextMenu_Impl for ContextMenuHandler_Impl {
    fn QueryContextMenu(
        &self,
        hmenu: HMENU,
        indexmenu: u32,
        idcmdfirst: u32,
        _idcmdlast: u32,
        uflags: u32,
    ) -> Result<()> {
        // If CMF_DEFAULTONLY is set, don't add our items.
        if (uflags & CMF_DEFAULTONLY) != 0 {
            return Ok(());
        }

        let state = self.state.lock().unwrap();

        // Only show menu items if at least one selected file has a sidecar.
        if !state.has_any_sidecar {
            return Ok(());
        }
        drop(state);

        // Store the first command ID for later use in InvokeCommand.
        self.state.lock().unwrap().id_cmd_first = idcmdfirst;

        // Insert "Copy with sidecar"
        let copy_text: Vec<u16> = MENU_COPY_TEXT
            .encode_utf16()
            .chain(core::iter::once(0))
            .collect();
        let copy_item = MENUITEMINFOW {
            cbSize: core::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_ID | MIIM_STRING | MIIM_FTYPE,
            fType: MFT_STRING,
            wID: idcmdfirst + CMD_COPY,
            dwTypeData: PWSTR(copy_text.as_ptr() as *mut _),
            cch: MENU_COPY_TEXT.len() as u32,
            ..Default::default()
        };
        unsafe { InsertMenuItemW(hmenu, indexmenu, true, &copy_item)? };

        // Insert "Move with sidecar"
        let move_text: Vec<u16> = MENU_MOVE_TEXT
            .encode_utf16()
            .chain(core::iter::once(0))
            .collect();
        let move_item = MENUITEMINFOW {
            cbSize: core::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_ID | MIIM_STRING | MIIM_FTYPE,
            fType: MFT_STRING,
            wID: idcmdfirst + CMD_MOVE,
            dwTypeData: PWSTR(move_text.as_ptr() as *mut _),
            cch: MENU_MOVE_TEXT.len() as u32,
            ..Default::default()
        };
        unsafe { InsertMenuItemW(hmenu, indexmenu + 1, true, &move_item)? };

        // QueryContextMenu must return the number of menu items added in
        // the low word of the HRESULT. windows-rs maps Ok(()) -> S_OK (0),
        // so we use Err with a success HRESULT to pass the count through.
        Err(Error::from(HRESULT(CMD_COUNT as i32)))
    }

    fn InvokeCommand(&self, pici: *const CMINVOKECOMMANDINFO) -> Result<()> {
        let pici = unsafe { &*pici };

        // lpVerb can be either a string verb or a command ID in the low word.
        // We only handle the numeric case.
        let verb = pici.lpVerb.0 as usize;
        if verb > 0xFFFF {
            // String verb -- not ours.
            return Err(Error::from(E_FAIL));
        }

        let state = self.state.lock().unwrap();
        let entries: Vec<_> = state
            .entries
            .iter()
            .map(|e| FileEntry {
                image: e.image.clone(),
                sidecar: e.sidecar.clone(),
            })
            .collect();
        drop(state);

        if entries.is_empty() {
            return Err(Error::from(E_FAIL));
        }

        let cmd_id = verb as u32;
        let is_move = match cmd_id {
            x if x == CMD_COPY => false,
            x if x == CMD_MOVE => true,
            _ => return Err(Error::from(E_FAIL)),
        };

        // Pick destination folder via IFileOpenDialog in folder-picker mode.
        let dest_folder = pick_folder(pici.hwnd)?;

        // Perform the file operation for all entries in a single IFileOperation batch.
        perform_file_op(&entries, &dest_folder, is_move)
    }

    fn GetCommandString(
        &self,
        idcmd: usize,
        utype: u32,
        _preserved: *const u32,
        pszname: PSTR,
        cchmax: u32,
    ) -> Result<()> {
        // GCS_HELPTEXTW = 0x00000005
        const GCS_HELPTEXTW: u32 = 0x00000005;

        if utype != GCS_HELPTEXTW {
            return Err(Error::from(E_NOTIMPL));
        }

        let help = match idcmd as u32 {
            CMD_COPY => "Copy selected files and their XMP sidecars to another folder",
            CMD_MOVE => "Move selected files and their XMP sidecars to another folder",
            _ => return Err(Error::from(E_INVALIDARG)),
        };

        let wide: Vec<u16> = help.encode_utf16().chain(core::iter::once(0)).collect();
        let copy_len = wide.len().min(cchmax as usize);
        unsafe {
            core::ptr::copy_nonoverlapping(
                wide.as_ptr(),
                pszname.0 as *mut u16,
                copy_len,
            );
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Folder picker dialog
// ---------------------------------------------------------------------------

fn pick_folder(hwnd_owner: HWND) -> Result<PathBuf> {
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;

        // Set folder-picker mode.
        let options = dialog.GetOptions()?;
        dialog.SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)?;

        dialog.SetTitle(&HSTRING::from("Select destination folder"))?;

        // Show the dialog.
        dialog.Show(hwnd_owner)?;

        let result: IShellItem = dialog.GetResult()?;
        let path_pwstr = result.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path_str = path_pwstr.to_string()?;
        CoTaskMemFree(Some(path_pwstr.0 as *const _));

        Ok(PathBuf::from(path_str))
    }
}

// ---------------------------------------------------------------------------
// File operation (copy or move) via IFileOperation
// ---------------------------------------------------------------------------

fn perform_file_op(
    entries: &[FileEntry],
    dest_folder: &Path,
    is_move: bool,
) -> Result<()> {
    unsafe {
        let file_op: IFileOperation =
            CoCreateInstance(&FileOperation, None, CLSCTX_ALL)?;

        // Allow undo + no confirmation for simple operations.
        file_op.SetOperationFlags(
            FOF_ALLOWUNDO | FOF_NOCONFIRMMKDIR,
        )?;

        // Create IShellItem for the destination folder.
        let dest_wide: Vec<u16> = dest_folder
            .to_string_lossy()
            .encode_utf16()
            .chain(core::iter::once(0))
            .collect();
        let dest_item: IShellItem =
            SHCreateItemFromParsingName(PCWSTR(dest_wide.as_ptr()), None)?;

        // Add all images (and their sidecars when present) to the operation.
        for entry in entries {
            add_file_to_op(&file_op, &entry.image, &dest_item, is_move)?;
            if let Some(ref sidecar) = entry.sidecar {
                add_file_to_op(&file_op, sidecar, &dest_item, is_move)?;
            }
        }

        file_op.PerformOperations()?;

        // Check if the user aborted mid-operation.
        if file_op.GetAnyOperationsAborted()?.as_bool() {
            return Err(Error::from(E_ABORT));
        }

        Ok(())
    }
}

unsafe fn add_file_to_op(
    file_op: &IFileOperation,
    path: &Path,
    dest_item: &IShellItem,
    is_move: bool,
) -> Result<()> {
    let wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(core::iter::once(0))
        .collect();
    let item: IShellItem =
        SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)?;

    if is_move {
        file_op.MoveItem(&item, dest_item, None, None)?;
    } else {
        file_op.CopyItem(&item, dest_item, None, None)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Class factory for the context menu handler
// ---------------------------------------------------------------------------

#[implement(IClassFactory)]
pub struct ContextMenuFactory;

impl IClassFactory_Impl for ContextMenuFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        unsafe {
            if ppvobject.is_null() {
                return Err(E_POINTER.into());
            }
            *ppvobject = core::ptr::null_mut();

            if punkouter.is_some() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }

            let handler = ContextMenuHandler::new();
            let unknown: IUnknown = handler.into();

            let this: *mut core::ffi::c_void = core::mem::transmute_copy(&unknown);
            let hr = (unknown.vtable().QueryInterface)(this, riid, ppvobject);
            hr.ok()
        }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}
