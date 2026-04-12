use std::path::Path;
use std::sync::Mutex;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::PropertiesSystem::*;
use windows_core::PROPVARIANT;

use crate::embedded;
use crate::pkeys::*;
use crate::sidecar::{self, XmpFields};

const STG_E_ACCESSDENIED: HRESULT = HRESULT(0x80030005_u32 as i32);

/// A property entry: the PKEY and the PROPVARIANT value to return for it.
pub struct PropEntry {
    pub key: PROPERTYKEY,
    pub value: PROPVARIANT,
}

/// Build the list of properties to expose from parsed XMP fields.
fn build_properties(fields: &XmpFields) -> Vec<PropEntry> {
    let mut props = Vec::new();

    if let Some(ref title) = fields.title {
        props.push(PropEntry {
            key: PKEY_TITLE,
            value: PROPVARIANT::from(BSTR::from(title.as_str())),
        });
    }

    if let Some(ref desc) = fields.description {
        props.push(PropEntry {
            key: PKEY_COMMENT,
            value: PROPVARIANT::from(BSTR::from(desc.as_str())),
        });
    }

    if !fields.keywords.is_empty() {
        // System.Keywords expects VT_VECTOR|VT_LPWSTR, but Explorer also
        // accepts a semicolon-separated VT_BSTR string.
        let joined = fields.keywords.join("; ");
        props.push(PropEntry {
            key: PKEY_KEYWORDS,
            value: PROPVARIANT::from(BSTR::from(joined.as_str())),
        });
    }

    if !fields.creators.is_empty() {
        let joined = fields.creators.join("; ");
        props.push(PropEntry {
            key: PKEY_AUTHOR,
            value: PROPVARIANT::from(BSTR::from(joined.as_str())),
        });
    }

    if let Some(stars) = fields.rating {
        let win_rating = xmp_rating_to_windows(stars);
        props.push(PropEntry {
            key: PKEY_RATING,
            value: PROPVARIANT::from(win_rating),
        });
    }

    if let Some(ref date) = fields.date_taken {
        props.push(PropEntry {
            key: PKEY_DATE_TAKEN,
            value: PROPVARIANT::from(BSTR::from(date.as_str())),
        });
    }

    if let Some(ref headline) = fields.headline {
        props.push(PropEntry {
            key: PKEY_PHOTO_EVENT,
            value: PROPVARIANT::from(BSTR::from(headline.as_str())),
        });
        props.push(PropEntry {
            key: PKEY_XMP_HEADLINE,
            value: PROPVARIANT::from(BSTR::from(headline.as_str())),
        });
    }

    if let Some(ref location) = fields.location {
        props.push(PropEntry {
            key: PKEY_XMP_LOCATION,
            value: PROPVARIANT::from(BSTR::from(location.as_str())),
        });
    }

    if !fields.person_in_image.is_empty() {
        let joined = fields.person_in_image.join("; ");
        props.push(PropEntry {
            key: PKEY_XMP_PERSON_IN_IMAGE,
            value: PROPVARIANT::from(BSTR::from(joined.as_str())),
        });
    }

    if let Some(ref place) = fields.photostat_place {
        props.push(PropEntry {
            key: PKEY_XMP_PLACE,
            value: PROPVARIANT::from(BSTR::from(place.as_str())),
        });
    }

    if !fields.photostat_cloud_uploads.is_empty() {
        let joined = fields.photostat_cloud_uploads.join("; ");
        props.push(PropEntry {
            key: PKEY_XMP_CLOUD_UPLOADS,
            value: PROPVARIANT::from(BSTR::from(joined.as_str())),
        });
    }

    props
}

// ---------------------------------------------------------------------------
// Property handler - the COM object Explorer creates per file
// ---------------------------------------------------------------------------

#[implement(IInitializeWithFile, IPropertyStore, IPropertyStoreCapabilities)]
pub struct PropertyHandler {
    state: Mutex<HandlerState>,
}

struct HandlerState {
    props: Vec<PropEntry>,
}

impl PropertyHandler {
    fn new() -> Self {
        Self {
            state: Mutex::new(HandlerState { props: Vec::new() }),
        }
    }
}

impl IInitializeWithFile_Impl for PropertyHandler_Impl {
    fn Initialize(&self, pszfilepath: &PCWSTR, _grfmode: u32) -> Result<()> {
        let path_str = unsafe { pszfilepath.to_string()? };
        let path = Path::new(&path_str);

        let mut state = self.state.lock().unwrap();

        // 1. Load embedded metadata from the old system handler as our base.
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_ascii_lowercase()))
            .unwrap_or_default();
        let mut props = embedded::load_embedded(&path_str, &ext);

        // 2. If a sidecar exists, overlay its properties (sidecar wins on conflict).
        if let Some(sidecar_path) = sidecar::find_sidecar(path) {
            if let Ok(fields) = sidecar::parse_sidecar(&sidecar_path) {
                let sidecar_props = build_properties(&fields);
                for sp in sidecar_props {
                    // Replace any existing entry with the same PKEY.
                    if let Some(existing) = props.iter_mut().find(|p| {
                        p.key.fmtid == sp.key.fmtid && p.key.pid == sp.key.pid
                    }) {
                        existing.value = sp.value;
                    } else {
                        props.push(sp);
                    }
                }
            }
        }

        state.props = props;

        Ok(())
    }
}

impl IPropertyStore_Impl for PropertyHandler_Impl {
    fn GetCount(&self) -> Result<u32> {
        let state = self.state.lock().unwrap();
        Ok(state.props.len() as u32)
    }

    fn GetAt(&self, iprop: u32, pkey: *mut PROPERTYKEY) -> Result<()> {
        if pkey.is_null() {
            return Err(E_INVALIDARG.into());
        }
        let state = self.state.lock().unwrap();
        match state.props.get(iprop as usize) {
            Some(entry) => {
                unsafe { *pkey = entry.key };
                Ok(())
            }
            None => Err(E_INVALIDARG.into()),
        }
    }

    fn GetValue(&self, key: *const PROPERTYKEY) -> Result<PROPVARIANT> {
        let key = unsafe { &*key };
        let state = self.state.lock().unwrap();
        for entry in &state.props {
            if entry.key.fmtid == key.fmtid && entry.key.pid == key.pid {
                // Clone the PROPVARIANT for the caller.
                return Ok(entry.value.clone());
            }
        }
        // Property not found - return empty.
        Ok(PROPVARIANT::default())
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
        Err(Error::from(S_FALSE))
    }
}

// ---------------------------------------------------------------------------
// Class factory
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

            let this: *mut core::ffi::c_void = core::mem::transmute_copy(&unknown);
            let hr = (unknown.vtable().QueryInterface)(this, riid, ppvobject);
            hr.ok()
        }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidecar::XmpFields;

    #[test]
    fn build_properties_full() {
        let fields = XmpFields {
            rating: Some(4),
            title: Some("Test Title".into()),
            description: Some("Test Desc".into()),
            keywords: vec!["a".into(), "b".into()],
            creators: vec!["Alice".into()],
            date_taken: Some("2025-06-15".into()),
            headline: Some("Headline".into()),
            location: None,
            person_in_image: Vec::new(),
            photostat_place: None,
            photostat_cloud_uploads: Vec::new(),
        };
        let props = build_properties(&fields);
        // 7 standard + 1 custom headline = 8
        assert_eq!(props.len(), 8);

        // Verify title is present
        assert!(props.iter().any(|p| p.key.fmtid == PKEY_TITLE.fmtid && p.key.pid == PKEY_TITLE.pid));
        // Verify rating is present
        assert!(props.iter().any(|p| p.key.fmtid == PKEY_RATING.fmtid && p.key.pid == PKEY_RATING.pid));
    }

    #[test]
    fn build_properties_empty() {
        let fields = XmpFields::default();
        let props = build_properties(&fields);
        assert!(props.is_empty());
    }

    #[test]
    fn merge_sidecar_overrides_embedded() {
        // Simulate embedded props with a title
        let mut props = vec![
            PropEntry {
                key: PKEY_TITLE,
                value: PROPVARIANT::from(BSTR::from("Embedded Title")),
            },
            PropEntry {
                key: PKEY_RATING,
                value: PROPVARIANT::from(75u32),
            },
        ];

        // Sidecar has a different title but no rating
        let sidecar_fields = XmpFields {
            title: Some("Sidecar Title".into()),
            ..Default::default()
        };
        let sidecar_props = build_properties(&sidecar_fields);

        // Merge: sidecar overrides matching keys, embedded-only keys survive
        for sp in sidecar_props {
            if let Some(existing) = props.iter_mut().find(|p| {
                p.key.fmtid == sp.key.fmtid && p.key.pid == sp.key.pid
            }) {
                existing.value = sp.value;
            } else {
                props.push(sp);
            }
        }

        // Should still have 2 entries (title replaced, rating preserved)
        assert_eq!(props.len(), 2);

        // Title should be from sidecar
        let title_entry = props.iter().find(|p| p.key.pid == PKEY_TITLE.pid).unwrap();
        let title_str = format!("{:?}", title_entry.value);
        assert!(title_str.contains("Sidecar Title"), "title should be from sidecar, got: {}", title_str);

        // Rating should be preserved from embedded
        assert!(props.iter().any(|p| p.key.fmtid == PKEY_RATING.fmtid && p.key.pid == PKEY_RATING.pid));
    }

    #[test]
    fn merge_sidecar_appends_new_keys() {
        // Embedded has only rating
        let mut props = vec![
            PropEntry {
                key: PKEY_RATING,
                value: PROPVARIANT::from(50u32),
            },
        ];

        // Sidecar adds title (not in embedded)
        let sidecar_fields = XmpFields {
            title: Some("New Title".into()),
            ..Default::default()
        };
        let sidecar_props = build_properties(&sidecar_fields);

        for sp in sidecar_props {
            if let Some(existing) = props.iter_mut().find(|p| {
                p.key.fmtid == sp.key.fmtid && p.key.pid == sp.key.pid
            }) {
                existing.value = sp.value;
            } else {
                props.push(sp);
            }
        }

        // Should have 2: original rating + new title
        assert_eq!(props.len(), 2);
        assert!(props.iter().any(|p| p.key.pid == PKEY_TITLE.pid));
        assert!(props.iter().any(|p| p.key.pid == PKEY_RATING.pid));
    }
}
