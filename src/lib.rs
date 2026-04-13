mod context_menu;
mod embedded;
mod handler;
mod pkeys;
mod registry;
mod sidecar;

use core::ffi::c_void;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;

use context_menu::ContextMenuFactory;
use handler::HandlerFactory;

/// CLSID for the XMP Sidecar Property Handler.
/// {D4B5A6C7-8E9F-4A1B-BC2D-3E4F5A6B7C8D}
pub const CLSID_XMP_HANDLER: GUID = GUID {
    data1: 0xD4B5A6C7,
    data2: 0x8E9F,
    data3: 0x4A1B,
    data4: [0xBC, 0x2D, 0x3E, 0x4F, 0x5A, 0x6B, 0x7C, 0x8D],
};

/// CLSID for the XMP Sidecar Context Menu extension.
/// {A1C2D3E4-5F60-4718-B9CA-0D1E2F3A4B5C}
pub const CLSID_XMP_CONTEXT_MENU: GUID = GUID {
    data1: 0xA1C2D3E4,
    data2: 0x5F60,
    data3: 0x4718,
    data4: [0xB9, 0xCA, 0x0D, 0x1E, 0x2F, 0x3A, 0x4B, 0x5C],
};

#[no_mangle]
unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();

    if rclsid.is_null() {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    // Only hand out IClassFactory or IUnknown.
    if *riid != IClassFactory::IID && *riid != IUnknown::IID {
        return E_NOINTERFACE;
    }

    if *rclsid == CLSID_XMP_HANDLER {
        let factory: IClassFactory = HandlerFactory.into();
        *ppv = core::mem::transmute(factory);
        return S_OK;
    }

    if *rclsid == CLSID_XMP_CONTEXT_MENU {
        let factory: IClassFactory = ContextMenuFactory.into();
        *ppv = core::mem::transmute(factory);
        return S_OK;
    }

    CLASS_E_CLASSNOTAVAILABLE
}

#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    // Keep the DLL loaded for the lifetime of the host process.
    S_FALSE
}

#[no_mangle]
extern "system" fn DllRegisterServer() -> HRESULT {
    match registry::register() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}

#[no_mangle]
extern "system" fn DllUnregisterServer() -> HRESULT {
    match registry::unregister() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}
