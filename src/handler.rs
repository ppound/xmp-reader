use std::sync::Mutex;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows_core::PROPVARIANT;
use windows::Win32::UI::Shell::PropertiesSystem::*;

/// System.Comment property key.
/// {F29F85E0-4FF9-1068-AB91-08002B27B3D9} pid 6
const PKEY_COMMENT: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0xF29F85E0,
        data2: 0x4FF9,
        data3: 0x1068,
        data4: [0xAB, 0x91, 0x08, 0x00, 0x2B, 0x27, 0xB3, 0xD9],
    },
    pid: 6,
};

const STG_E_ACCESSDENIED: HRESULT = HRESULT(0x80030005_u32 as i32);

// ---------------------------------------------------------------------------
// Property handler - the COM object Explorer creates per file
// ---------------------------------------------------------------------------

#[implement(IInitializeWithFile, IPropertyStore, IPropertyStoreCapabilities)]
pub struct PropertyHandler {
    path: Mutex<Option<String>>,
}

impl PropertyHandler {
    fn new() -> Self {
        Self {
            path: Mutex::new(None),
        }
    }
}

impl IInitializeWithFile_Impl for PropertyHandler_Impl {
    fn Initialize(&self, pszfilepath: &PCWSTR, _grfmode: u32) -> Result<()> {
        let s = unsafe { pszfilepath.to_string()? };
        *self.path.lock().unwrap() = Some(s);
        Ok(())
    }
}

impl IPropertyStore_Impl for PropertyHandler_Impl {
    fn GetCount(&self) -> Result<u32> {
        Ok(1)
    }

    fn GetAt(&self, iprop: u32, pkey: *mut PROPERTYKEY) -> Result<()> {
        if iprop != 0 || pkey.is_null() {
            return Err(E_INVALIDARG.into());
        }
        unsafe { *pkey = PKEY_COMMENT };
        Ok(())
    }

    fn GetValue(&self, key: *const PROPERTYKEY) -> Result<PROPVARIANT> {
        let key = unsafe { &*key };
        if key.fmtid == PKEY_COMMENT.fmtid && key.pid == PKEY_COMMENT.pid {
            let value = BSTR::from("XMP sidecar handler active");
            Ok(PROPVARIANT::from(value))
        } else {
            Ok(PROPVARIANT::default())
        }
    }

    fn SetValue(&self, _key: *const PROPERTYKEY, _propvar: *const PROPVARIANT) -> Result<()> {
        Err(Error::from(STG_E_ACCESSDENIED))
    }

    fn Commit(&self) -> Result<()> {
        Err(Error::from(STG_E_ACCESSDENIED))
    }
}

impl IPropertyStoreCapabilities_Impl for PropertyHandler_Impl {
    fn IsPropertyWritable(&self, _key: *const PROPERTYKEY) -> Result<()> {
        // S_FALSE = not writable. The macro converts Err(S_FALSE) to HRESULT(1).
        Err(Error::from(S_FALSE))
    }
}

// ---------------------------------------------------------------------------
// Class factory - COM asks for this via DllGetClassObject
// ---------------------------------------------------------------------------

#[implement(IClassFactory)]
pub struct HandlerFactory;

impl IClassFactory_Impl for HandlerFactory_Impl {
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

            let handler = PropertyHandler::new();
            let unknown: IUnknown = handler.into();

            // QueryInterface for the requested interface. QI AddRefs for ppvobject;
            // unknown drops afterwards (Release), leaving the caller with one ref.
            let this: *mut core::ffi::c_void = core::mem::transmute_copy(&unknown);
            let hr = (unknown.vtable().QueryInterface)(this, riid, ppvobject);
            hr.ok()
        }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}
