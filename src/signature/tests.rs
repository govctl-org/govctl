use super::canonical_json::canonicalize_json;
use super::*;
use serde_json::Value;

#[test]
fn test_canonicalize_sorts_keys() -> Result<(), Box<dyn std::error::Error>> {
    let json: Value = serde_json::from_str(r#"{"z": 1, "a": 2, "m": 3}"#)?;
    let canonical = canonicalize_json(&json);
    assert_eq!(canonical, r#"{"a":2,"m":3,"z":1}"#);
    Ok(())
}

#[test]
fn test_canonicalize_nested_objects() -> Result<(), Box<dyn std::error::Error>> {
    let json: Value = serde_json::from_str(r#"{"outer": {"z": 1, "a": 2}, "inner": {"b": 3}}"#)?;
    let canonical = canonicalize_json(&json);
    assert_eq!(canonical, r#"{"inner":{"b":3},"outer":{"a":2,"z":1}}"#);
    Ok(())
}

#[test]
fn test_extract_signature() {
    let md = r#"---
status: normative
---

<!-- GENERATED: do not edit. Source: RFC-0000 -->
<!-- SIGNATURE: sha256:abcd1234 -->

# RFC-0000
"#;
    assert_eq!(extract_signature(md), Some("abcd1234".to_string()));
}

#[test]
fn test_extract_signature_not_found() {
    let md = "# Just a plain markdown file";
    assert_eq!(extract_signature(md), None);
}
