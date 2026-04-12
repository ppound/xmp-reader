use std::fs;
use std::path::{Path, PathBuf};

use xmp_toolkit::{xmp_ns, XmpMeta};

/// Extracted XMP sidecar fields, ready for mapping to Windows PKEYs in M3.
#[derive(Debug, Default, PartialEq)]
pub struct XmpFields {
    pub rating: Option<i32>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub creators: Vec<String>,
    pub date_taken: Option<String>,
    pub headline: Option<String>,
    pub location: Option<String>,
}

/// Find the XMP sidecar for `image_path`.
///
/// Checks (in order):
///   1. `<stem>.xmp`       — e.g. `photo.xmp` for `photo.jpg`
///   2. `<name>.xmp`       — e.g. `photo.jpg.xmp` (ExifTool convention)
///
/// Returns the first path that exists, or `None`.
pub fn find_sidecar(image_path: &Path) -> Option<PathBuf> {
    let dir = image_path.parent()?;
    let stem = image_path.file_stem()?.to_str()?;
    let name = image_path.file_name()?.to_str()?;

    // 1. <stem>.xmp
    let candidate = dir.join(format!("{stem}.xmp"));
    if candidate.is_file() {
        return Some(candidate);
    }

    // 2. <name>.xmp  (e.g. photo.jpg.xmp)
    let candidate = dir.join(format!("{name}.xmp"));
    if candidate.is_file() {
        return Some(candidate);
    }

    None
}

/// Parse an XMP sidecar file and extract the fields we care about.
pub fn parse_sidecar(sidecar_path: &Path) -> Result<XmpFields, String> {
    let xml = fs::read_to_string(sidecar_path)
        .map_err(|e| format!("failed to read {}: {e}", sidecar_path.display()))?;
    parse_xmp(&xml)
}

/// Parse raw XMP XML into an `XmpFields` struct.
pub fn parse_xmp(xml: &str) -> Result<XmpFields, String> {
    let xmp: XmpMeta = xml.parse().map_err(|e| format!("XMP parse error: {e}"))?;
    let mut fields = XmpFields::default();

    // --- Simple properties ---

    // xmp:Rating (integer)
    if let Some(val) = xmp.property(xmp_ns::XMP, "Rating") {
        fields.rating = val.value.parse().ok();
    }

    // photoshop:Headline
    if let Some(val) = xmp.property(xmp_ns::PHOTOSHOP, "Headline") {
        if !val.value.is_empty() {
            fields.headline = Some(val.value.clone());
        }
    }

    // Iptc4xmpCore:Location
    if let Some(val) = xmp.property("http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/", "Location") {
        if !val.value.is_empty() {
            fields.location = Some(val.value.clone());
        }
    }

    // Date taken: prefer photoshop:DateCreated, fall back to xmp:CreateDate
    if let Some(val) = xmp.property(xmp_ns::PHOTOSHOP, "DateCreated") {
        if !val.value.is_empty() {
            fields.date_taken = Some(val.value.clone());
        }
    }
    if fields.date_taken.is_none() {
        if let Some(val) = xmp.property(xmp_ns::XMP, "CreateDate") {
            if !val.value.is_empty() {
                fields.date_taken = Some(val.value.clone());
            }
        }
    }

    // --- Lang Alt properties (must use localized_text) ---

    // dc:title
    if let Some((val, _)) = xmp.localized_text(xmp_ns::DC, "title", None, "x-default") {
        if !val.value.is_empty() {
            fields.title = Some(val.value.clone());
        }
    }

    // dc:description
    if let Some((val, _)) = xmp.localized_text(xmp_ns::DC, "description", None, "x-default") {
        if !val.value.is_empty() {
            fields.description = Some(val.value.clone());
        }
    }

    // --- Array properties ---

    // dc:subject (unordered bag -> keywords)
    fields.keywords = iter_array(&xmp, xmp_ns::DC, "subject");

    // dc:creator (ordered seq -> authors)
    fields.creators = iter_array(&xmp, xmp_ns::DC, "creator");

    Ok(fields)
}

/// Iterate an XMP array property (bag or seq) by index, returning all string values.
fn iter_array(xmp: &XmpMeta, ns: &str, name: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut i = 1;
    loop {
        let path = format!("{name}[{i}]");
        match xmp.property(ns, &path) {
            Some(val) if !val.value.is_empty() => {
                items.push(val.value.clone());
                i += 1;
            }
            _ => break,
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const FULL_XMP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description
        xmlns:dc="http://purl.org/dc/elements/1.1/"
        xmlns:xmp="http://ns.adobe.com/xap/1.0/"
        xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/"
        xmlns:Iptc4xmpCore="http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/"
        xmp:Rating="4"
        photoshop:Headline="Test Headline"
        photoshop:DateCreated="2025-06-15T10:30:00"
        Iptc4xmpCore:Location="Test Location">
      <dc:title>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Test Title</rdf:li>
        </rdf:Alt>
      </dc:title>
      <dc:description>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Test Description</rdf:li>
        </rdf:Alt>
      </dc:description>
      <dc:subject>
        <rdf:Bag>
          <rdf:li>landscape</rdf:li>
          <rdf:li>nature</rdf:li>
          <rdf:li>sunset</rdf:li>
        </rdf:Bag>
      </dc:subject>
      <dc:creator>
        <rdf:Seq>
          <rdf:li>Alice</rdf:li>
          <rdf:li>Bob</rdf:li>
        </rdf:Seq>
      </dc:creator>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;

    const MINIMAL_XMP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description
        xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:title>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Minimal Title</rdf:li>
        </rdf:Alt>
      </dc:title>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;

    #[test]
    fn parse_full_xmp() {
        let fields = parse_xmp(FULL_XMP).unwrap();
        assert_eq!(fields.rating, Some(4));
        assert_eq!(fields.title.as_deref(), Some("Test Title"));
        assert_eq!(fields.description.as_deref(), Some("Test Description"));
        assert_eq!(fields.keywords, vec!["landscape", "nature", "sunset"]);
        assert_eq!(fields.creators, vec!["Alice", "Bob"]);
        assert_eq!(fields.date_taken.as_deref(), Some("2025-06-15T10:30:00"));
        assert_eq!(fields.headline.as_deref(), Some("Test Headline"));
        assert_eq!(fields.location.as_deref(), Some("Test Location"));
    }

    #[test]
    fn parse_minimal_xmp() {
        let fields = parse_xmp(MINIMAL_XMP).unwrap();
        assert_eq!(fields.title.as_deref(), Some("Minimal Title"));
        assert_eq!(fields.rating, None);
        assert_eq!(fields.description, None);
        assert!(fields.keywords.is_empty());
        assert!(fields.creators.is_empty());
        assert_eq!(fields.date_taken, None);
        assert_eq!(fields.headline, None);
        assert_eq!(fields.location, None);
    }

    #[test]
    fn find_sidecar_stem() {
        let dir = TempDir::new().unwrap();
        let jpg = dir.path().join("photo.jpg");
        let xmp = dir.path().join("photo.xmp");
        fs::File::create(&jpg).unwrap();
        fs::write(&xmp, MINIMAL_XMP).unwrap();

        assert_eq!(find_sidecar(&jpg), Some(xmp));
    }

    #[test]
    fn find_sidecar_ext_xmp() {
        let dir = TempDir::new().unwrap();
        let jpg = dir.path().join("photo.jpg");
        let xmp = dir.path().join("photo.jpg.xmp");
        fs::File::create(&jpg).unwrap();
        fs::write(&xmp, MINIMAL_XMP).unwrap();

        assert_eq!(find_sidecar(&jpg), Some(xmp));
    }

    #[test]
    fn find_sidecar_prefers_stem() {
        let dir = TempDir::new().unwrap();
        let jpg = dir.path().join("photo.jpg");
        let xmp_stem = dir.path().join("photo.xmp");
        let _xmp_ext = dir.path().join("photo.jpg.xmp");
        fs::File::create(&jpg).unwrap();
        fs::write(&xmp_stem, MINIMAL_XMP).unwrap();
        fs::write(&_xmp_ext, MINIMAL_XMP).unwrap();

        assert_eq!(find_sidecar(&jpg), Some(xmp_stem));
    }

    #[test]
    fn find_sidecar_none() {
        let dir = TempDir::new().unwrap();
        let jpg = dir.path().join("photo.jpg");
        fs::File::create(&jpg).unwrap();

        assert_eq!(find_sidecar(&jpg), None);
    }
}
