use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::PropertiesSystem::{
    PSRegisterPropertySchema, PSUnregisterPropertySchema,
};

use crate::CLSID_XMP_HANDLER;

const HANDLER_DESCRIPTION: &str = "XMP Sidecar Property Handler";
const HANDLER_BASE: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\PropertySystem\PropertyHandlers\";

const EXTENSIONS: &[&str] = &[".jpg", ".cr2", ".nef", ".arw", ".dng", ".raf", ".tif", ".tiff"];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn guid_to_string(g: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        g.data1,
        g.data2,
        g.data3,
        g.data4[0],
        g.data4[1],
        g.data4[2],
        g.data4[3],
        g.data4[4],
        g.data4[5],
        g.data4[6],
        g.data4[7],
    )
}

fn get_dll_path() -> Result<String> {
    unsafe {
        let mut hmod = HMODULE::default();
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(get_dll_path as *const () as usize as *const u16),
            &mut hmod,
        )?;
        let mut buf = vec![0u16; 32768];
        let len = GetModuleFileNameW(hmod, &mut buf[..]);
        if len == 0 || len as usize >= buf.len() {
            return Err(Error::from_win32());
        }
        Ok(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

fn get_propdesc_path() -> Result<String> {
    let dll = get_dll_path()?;
    // Replace the DLL filename with the .propdesc filename.
    match dll.rfind('\\') {
        Some(pos) => Ok(format!("{}\\xmp-sidecar.propdesc", &dll[..pos])),
        None => Err(Error::from(E_FAIL)),
    }
}

/// Null-terminated UTF-16 string.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(core::iter::once(0)).collect()
}

fn create_key(parent: HKEY, subkey: &str, access: REG_SAM_FLAGS) -> Result<HKEY> {
    let w = wide(subkey);
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            parent,
            PCWSTR(w.as_ptr()),
            0,
            PCWSTR(core::ptr::null()), // lpClass = NULL
            REG_OPTION_NON_VOLATILE,
            access,
            None, // security attributes
            &mut key,
            None, // disposition
        )
        .ok()?;
    }
    Ok(key)
}

fn set_string(key: HKEY, name: Option<&str>, value: &str) -> Result<()> {
    let name_w = name.map(wide);
    let name_pcwstr = match &name_w {
        Some(w) => PCWSTR(w.as_ptr()),
        None => PCWSTR(core::ptr::null()),
    };
    let val_w = wide(value);
    let bytes = unsafe {
        core::slice::from_raw_parts(val_w.as_ptr() as *const u8, val_w.len() * 2)
    };
    unsafe { RegSetValueExW(key, name_pcwstr, 0, REG_SZ, Some(bytes)).ok() }
}

fn get_string(key: HKEY, name: Option<&str>) -> Result<String> {
    let name_w = name.map(wide);
    let name_pcwstr = match &name_w {
        Some(w) => PCWSTR(w.as_ptr()),
        None => PCWSTR(core::ptr::null()),
    };
    unsafe {
        let mut size: u32 = 0;
        // First call: get buffer size.
        let _ = RegQueryValueExW(
            key,
            name_pcwstr,
            None,
            None,
            None,
            Some(&mut size as *mut u32),
        );
        if size == 0 {
            return Ok(String::new());
        }
        let mut buf = vec![0u8; size as usize];
        RegQueryValueExW(
            key,
            name_pcwstr,
            None,
            None,
            Some(buf.as_mut_ptr()),
            Some(&mut size as *mut u32),
        )
        .ok()?;
        let chars =
            core::slice::from_raw_parts(buf.as_ptr() as *const u16, size as usize / 2);
        let len = chars.iter().position(|&c| c == 0).unwrap_or(chars.len());
        Ok(String::from_utf16_lossy(&chars[..len]))
    }
}

/// Parse a `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}` string into a GUID.
pub fn parse_guid(s: &str) -> Option<GUID> {
    let s = s.trim_matches(|c| c == '{' || c == '}');
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return None;
    }
    let data1 = u32::from_str_radix(parts[0], 16).ok()?;
    let data2 = u16::from_str_radix(parts[1], 16).ok()?;
    let data3 = u16::from_str_radix(parts[2], 16).ok()?;
    let hi = u16::from_str_radix(parts[3], 16).ok()?;
    let lo = u64::from_str_radix(parts[4], 16).ok()?;
    let data4 = [
        (hi >> 8) as u8,
        hi as u8,
        (lo >> 40) as u8,
        (lo >> 32) as u8,
        (lo >> 24) as u8,
        (lo >> 16) as u8,
        (lo >> 8) as u8,
        lo as u8,
    ];
    Some(GUID { data1, data2, data3, data4 })
}

/// Read the old (system) property handler CLSID saved during registration for the given extension.
pub fn get_old_handler_clsid(ext: &str) -> Option<GUID> {
    let path = format!("{}{}", HANDLER_BASE, ext);
    let hk = create_key(HKEY_LOCAL_MACHINE, &path, KEY_READ).ok()?;
    let val = get_string(hk, Some("OldHandler")).ok()?;
    unsafe { let _ = RegCloseKey(hk); }
    if val.is_empty() {
        return None;
    }
    parse_guid(&val)
}

// ---------------------------------------------------------------------------
// Public API called from DllRegisterServer / DllUnregisterServer
// ---------------------------------------------------------------------------

pub fn register() -> Result<()> {
    let dll_path = get_dll_path()?;
    let clsid = guid_to_string(&CLSID_XMP_HANDLER);

    // 1. HKCR\CLSID\{guid}
    let clsid_key = create_key(
        HKEY_CLASSES_ROOT,
        &format!(r"CLSID\{clsid}"),
        KEY_WRITE,
    )?;
    set_string(clsid_key, None, HANDLER_DESCRIPTION)?;

    // 2. HKCR\CLSID\{guid}\InprocServer32
    let inproc_key = create_key(
        HKEY_CLASSES_ROOT,
        &format!(r"CLSID\{clsid}\InprocServer32"),
        KEY_WRITE,
    )?;
    set_string(inproc_key, None, &dll_path)?;
    set_string(inproc_key, Some("ThreadingModel"), "Both")?;

    unsafe {
        let _ = RegCloseKey(inproc_key);
        let _ = RegCloseKey(clsid_key);
    }

    // 3. Register as property handler for every supported extension.
    for ext in EXTENSIONS {
        let ext_path = format!("{}{}", HANDLER_BASE, ext);
        let hk = create_key(HKEY_LOCAL_MACHINE, &ext_path, KEY_READ | KEY_WRITE)?;

        if let Ok(existing) = get_string(hk, None) {
            if !existing.is_empty() && existing != clsid {
                let _ = set_string(hk, Some("OldHandler"), &existing);
            }
        }
        set_string(hk, None, &clsid)?;

        unsafe {
            let _ = RegCloseKey(hk);
        }
    }

    // 4. Register custom property schema.
    if let Ok(schema_path) = get_propdesc_path() {
        let w = wide(&schema_path);
        unsafe {
            let _ = PSRegisterPropertySchema(PCWSTR(w.as_ptr()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_guid_valid() {
        let g = parse_guid("{D4B5A6C7-8E9F-4A1B-BC2D-3E4F5A6B7C8D}").unwrap();
        assert_eq!(g.data1, 0xD4B5A6C7);
        assert_eq!(g.data2, 0x8E9F);
        assert_eq!(g.data3, 0x4A1B);
        assert_eq!(g.data4, [0xBC, 0x2D, 0x3E, 0x4F, 0x5A, 0x6B, 0x7C, 0x8D]);
    }

    #[test]
    fn parse_guid_without_braces() {
        let g = parse_guid("D4B5A6C7-8E9F-4A1B-BC2D-3E4F5A6B7C8D").unwrap();
        assert_eq!(g.data1, 0xD4B5A6C7);
    }

    #[test]
    fn parse_guid_invalid_returns_none() {
        assert!(parse_guid("not-a-guid").is_none());
        assert!(parse_guid("").is_none());
        assert!(parse_guid("{ZZZZZZZZ-8E9F-4A1B-BC2D-3E4F5A6B7C8D}").is_none());
    }

    #[test]
    fn guid_roundtrip() {
        let original = "{D4B5A6C7-8E9F-4A1B-BC2D-3E4F5A6B7C8D}";
        let g = parse_guid(original).unwrap();
        let formatted = guid_to_string(&g);
        assert_eq!(formatted, original);
    }

    #[test]
    fn guid_to_string_format() {
        let g = GUID {
            data1: 0xA38B883C,
            data2: 0x1682,
            data3: 0x497E,
            data4: [0x97, 0xB0, 0x0A, 0x3A, 0x9E, 0x80, 0x16, 0x82],
        };
        assert_eq!(
            guid_to_string(&g),
            "{A38B883C-1682-497E-97B0-0A3A9E801682}"
        );
    }

    #[test]
    fn extensions_list_contains_expected() {
        assert!(EXTENSIONS.contains(&".jpg"));
        assert!(EXTENSIONS.contains(&".cr2"));
        assert!(EXTENSIONS.contains(&".nef"));
        assert!(EXTENSIONS.contains(&".arw"));
        assert!(EXTENSIONS.contains(&".dng"));
        assert!(EXTENSIONS.contains(&".raf"));
        assert!(EXTENSIONS.contains(&".tif"));
        assert!(EXTENSIONS.contains(&".tiff"));
    }
}

pub fn unregister() -> Result<()> {
    let clsid = guid_to_string(&CLSID_XMP_HANDLER);

    // 0. Unregister custom property schema.
    if let Ok(schema_path) = get_propdesc_path() {
        let w = wide(&schema_path);
        unsafe {
            let _ = PSUnregisterPropertySchema(PCWSTR(w.as_ptr()));
        }
    }

    // 1. Restore old handler for every supported extension.
    for ext in EXTENSIONS {
        let ext_path = format!("{}{}", HANDLER_BASE, ext);
        if let Ok(hk) = create_key(HKEY_LOCAL_MACHINE, &ext_path, KEY_READ | KEY_WRITE) {
            if let Ok(old) = get_string(hk, Some("OldHandler")) {
                if !old.is_empty() {
                    let _ = set_string(hk, None, &old);
                    let w = wide("OldHandler");
                    unsafe {
                        let _ = RegDeleteValueW(hk, PCWSTR(w.as_ptr()));
                    }
                } else {
                    unsafe {
                        let _ = RegDeleteValueW(hk, PCWSTR(core::ptr::null()));
                    }
                }
            }
            unsafe {
                let _ = RegCloseKey(hk);
            }
        }
    }

    // 2. Remove our CLSID key tree.
    let w = wide(&format!(r"CLSID\{clsid}"));
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(w.as_ptr()));
    }

    Ok(())
}
