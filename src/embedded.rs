//! Read embedded metadata from an image file by delegating to the old (system)
//! property handler that we displaced during registration.

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::PropertiesSystem::*;
use windows::Win32::UI::Shell::SHCreateStreamOnFileEx;
use windows_core::PROPVARIANT;

use crate::handler::PropEntry;
use crate::registry;

/// Load all properties from the old system handler for the given file.
/// Returns an empty vec (not an error) if the old handler is unavailable.
pub fn load_embedded(file_path: &str) -> Vec<PropEntry> {
    match try_load_embedded(file_path) {
        Ok(props) => props,
        Err(_) => Vec::new(),
    }
}

fn try_load_embedded(file_path: &str) -> Result<Vec<PropEntry>> {
    let old_clsid = registry::get_old_handler_clsid()
        .ok_or_else(|| Error::from(E_FAIL))?;

    // Create an instance of the old handler.
    let store: IPropertyStore = unsafe {
        CoCreateInstance(&old_clsid, None, CLSCTX_INPROC_SERVER)?
    };

    // Initialize it. The system JPEG handler typically implements
    // IInitializeWithStream, not IInitializeWithFile.
    let wide_path: Vec<u16> = file_path.encode_utf16().chain(core::iter::once(0)).collect();

    if let Ok(init) = store.cast::<IInitializeWithFile>() {
        unsafe { init.Initialize(PCWSTR(wide_path.as_ptr()), 0)? };
    } else if let Ok(init) = store.cast::<IInitializeWithStream>() {
        let stream = unsafe {
            SHCreateStreamOnFileEx(
                PCWSTR(wide_path.as_ptr()),
                STGM_READ.0,
                0,     // dwAttributes
                FALSE, // fCreate
                None,  // pstmTemplate
            )?
        };
        unsafe { init.Initialize(&stream, STGM_READ.0)? };
    } else {
        return Err(Error::from(E_NOINTERFACE));
    }

    // Read all properties from the old handler.
    let count = unsafe { store.GetCount()? };
    let mut props = Vec::with_capacity(count as usize);

    for i in 0..count {
        let mut key = PROPERTYKEY::default();
        if unsafe { store.GetAt(i, &mut key) }.is_err() {
            continue;
        }
        let value = match unsafe { store.GetValue(&key) } {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Skip empty/VT_EMPTY values.
        if value == PROPVARIANT::default() {
            continue;
        }
        props.push(PropEntry { key, value });
    }

    Ok(props)
}
