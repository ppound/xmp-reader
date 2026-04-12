use windows::core::GUID;
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

// System.Rating  {64440492-4C8B-11D1-8B70-080036B11A03} pid 9
pub const PKEY_RATING: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0x64440492,
        data2: 0x4C8B,
        data3: 0x11D1,
        data4: [0x8B, 0x70, 0x08, 0x00, 0x36, 0xB1, 0x1A, 0x03],
    },
    pid: 9,
};

// System.Title  {F29F85E0-4FF9-1068-AB91-08002B27B3D9} pid 2
pub const PKEY_TITLE: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0xF29F85E0,
        data2: 0x4FF9,
        data3: 0x1068,
        data4: [0xAB, 0x91, 0x08, 0x00, 0x2B, 0x27, 0xB3, 0xD9],
    },
    pid: 2,
};

// System.Comment  {F29F85E0-4FF9-1068-AB91-08002B27B3D9} pid 6
pub const PKEY_COMMENT: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0xF29F85E0,
        data2: 0x4FF9,
        data3: 0x1068,
        data4: [0xAB, 0x91, 0x08, 0x00, 0x2B, 0x27, 0xB3, 0xD9],
    },
    pid: 6,
};

// System.Keywords  {F29F85E0-4FF9-1068-AB91-08002B27B3D9} pid 5
pub const PKEY_KEYWORDS: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0xF29F85E0,
        data2: 0x4FF9,
        data3: 0x1068,
        data4: [0xAB, 0x91, 0x08, 0x00, 0x2B, 0x27, 0xB3, 0xD9],
    },
    pid: 5,
};

// System.Author  {F29F85E0-4FF9-1068-AB91-08002B27B3D9} pid 4
pub const PKEY_AUTHOR: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0xF29F85E0,
        data2: 0x4FF9,
        data3: 0x1068,
        data4: [0xAB, 0x91, 0x08, 0x00, 0x2B, 0x27, 0xB3, 0xD9],
    },
    pid: 4,
};

// System.Photo.DateTaken  {14B81DA1-0135-4D31-96D9-6CBFC9671A99} pid 36867
pub const PKEY_DATE_TAKEN: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0x14B81DA1,
        data2: 0x0135,
        data3: 0x4D31,
        data4: [0x96, 0xD9, 0x6C, 0xBF, 0xC9, 0x67, 0x1A, 0x99],
    },
    pid: 36867,
};

// System.Photo.Event  {14B81DA1-0135-4D31-96D9-6CBFC9671A99} pid 18248
pub const PKEY_PHOTO_EVENT: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID {
        data1: 0x14B81DA1,
        data2: 0x0135,
        data3: 0x4D31,
        data4: [0x96, 0xD9, 0x6C, 0xBF, 0xC9, 0x67, 0x1A, 0x99],
    },
    pid: 18248,
};

// ---------------------------------------------------------------------------
// Custom properties (registered via .propdesc schema)
// Format ID: {B2A7E62A-1D9C-4F5E-8A3B-7C6D5E4F3A2B}
// ---------------------------------------------------------------------------

const CUSTOM_FMTID: GUID = GUID {
    data1: 0xB2A7E62A,
    data2: 0x1D9C,
    data3: 0x4F5E,
    data4: [0x8A, 0x3B, 0x7C, 0x6D, 0x5E, 0x4F, 0x3A, 0x2B],
};

// XmpSidecar.Headline  pid 2
pub const PKEY_XMP_HEADLINE: PROPERTYKEY = PROPERTYKEY {
    fmtid: CUSTOM_FMTID,
    pid: 2,
};

// XmpSidecar.Location  pid 3
pub const PKEY_XMP_LOCATION: PROPERTYKEY = PROPERTYKEY {
    fmtid: CUSTOM_FMTID,
    pid: 3,
};

// XmpSidecar.PersonInImage  pid 4
pub const PKEY_XMP_PERSON_IN_IMAGE: PROPERTYKEY = PROPERTYKEY {
    fmtid: CUSTOM_FMTID,
    pid: 4,
};

// XmpSidecar.Place  pid 5
pub const PKEY_XMP_PLACE: PROPERTYKEY = PROPERTYKEY {
    fmtid: CUSTOM_FMTID,
    pid: 5,
};

// XmpSidecar.CloudUploads  pid 6
pub const PKEY_XMP_CLOUD_UPLOADS: PROPERTYKEY = PROPERTYKEY {
    fmtid: CUSTOM_FMTID,
    pid: 6,
};

// ---------------------------------------------------------------------------

// System.Photo.LocationName  (undocumented but commonly used)
// Actually this is in the Photo PKEY set but not widely documented.
// Use the GPS location display name instead:
// System.GPS.LocationDescription  ... not standard either.
// Let's use System.Subject {D5CDD502-2E9C-101B-9397-08002B2CF9AE} pid 26 for now
// Actually, let's skip LocationName for M3 and focus on the well-known PKEYs.

/// Convert XMP star rating (1-5) to Windows System.Rating (0-99) scale.
/// The mapping matches Windows Photo Gallery / Windows Explorer conventions:
///   1 star -> 1,  2 stars -> 25,  3 stars -> 50,  4 stars -> 75,  5 stars -> 99
pub fn xmp_rating_to_windows(stars: i32) -> u32 {
    match stars {
        1 => 1,
        2 => 25,
        3 => 50,
        4 => 75,
        5 => 99,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rating_conversion_all_stars() {
        assert_eq!(xmp_rating_to_windows(1), 1);
        assert_eq!(xmp_rating_to_windows(2), 25);
        assert_eq!(xmp_rating_to_windows(3), 50);
        assert_eq!(xmp_rating_to_windows(4), 75);
        assert_eq!(xmp_rating_to_windows(5), 99);
    }

    #[test]
    fn rating_conversion_out_of_range() {
        assert_eq!(xmp_rating_to_windows(0), 0);
        assert_eq!(xmp_rating_to_windows(-1), 0);
        assert_eq!(xmp_rating_to_windows(6), 0);
    }

    #[test]
    fn pkey_title_and_comment_share_fmtid() {
        assert_eq!(PKEY_TITLE.fmtid, PKEY_COMMENT.fmtid);
        assert_ne!(PKEY_TITLE.pid, PKEY_COMMENT.pid);
    }

    #[test]
    fn pkey_date_taken_and_event_share_fmtid() {
        assert_eq!(PKEY_DATE_TAKEN.fmtid, PKEY_PHOTO_EVENT.fmtid);
        assert_ne!(PKEY_DATE_TAKEN.pid, PKEY_PHOTO_EVENT.pid);
    }
}
