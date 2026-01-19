//! Deterministic signature computation for rendered projections.
//!
//! Implements [[ADR-0003]] deterministic hash signatures.
//!
//! Signatures ensure rendered markdown files are read-only projections
//! of the authoritative JSON/TOML sources. Any direct edit to the markdown
//! will break the signature, which is detected by `govctl check`.
//!
//! Hash is SHA-256 of canonicalized content with sorted keys.

use crate::model::{AdrEntry, RfcIndex, WorkItemEntry};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Signature format version (for forward compatibility)
const SIGNATURE_VERSION: u32 = 1;

/// Compute SHA-256 signature for an RFC and its clauses.
///
/// The hash is computed over:
/// 1. Signature version prefix
/// 2. Canonical RFC JSON (keys sorted recursively)
/// 3. Canonical clause JSONs (sorted by clause_id, then keys sorted recursively)
///
/// # Errors
/// Returns an error if serialization fails (should not happen for valid specs).
pub fn compute_rfc_signature(rfc: &RfcIndex) -> Result<String, serde_json::Error> {
    let mut hasher = Sha256::new();

    // Version prefix for forward compatibility
    hasher.update(format!("govctl-signature-v{SIGNATURE_VERSION}\n").as_bytes());
    hasher.update(b"type:rfc\n");

    // Canonical RFC metadata
    let rfc_json = serde_json::to_value(&rfc.rfc)?;
    let canonical_rfc = canonicalize_json(&rfc_json);
    hasher.update(canonical_rfc.as_bytes());
    hasher.update(b"\n");

    // Sort clauses by clause_id for determinism
    let mut clauses: Vec<_> = rfc.clauses.iter().collect();
    clauses.sort_by(|a, b| a.spec.clause_id.cmp(&b.spec.clause_id));

    // Canonical clause content
    for clause in clauses {
        let clause_json = serde_json::to_value(&clause.spec)?;
        let canonical_clause = canonicalize_json(&clause_json);
        hasher.update(canonical_clause.as_bytes());
        hasher.update(b"\n");
    }

    // Produce hex string
    let digest = hasher.finalize();
    Ok(hex_encode(&digest))
}

/// Compute SHA-256 signature for an ADR.
///
/// # Errors
/// Returns an error if serialization fails (should not happen for valid specs).
pub fn compute_adr_signature(adr: &AdrEntry) -> Result<String, serde_json::Error> {
    let mut hasher = Sha256::new();

    hasher.update(format!("govctl-signature-v{SIGNATURE_VERSION}\n").as_bytes());
    hasher.update(b"type:adr\n");

    // Serialize TOML to JSON Value for canonical representation
    let adr_json = serde_json::to_value(&adr.spec)?;
    let canonical = canonicalize_json(&adr_json);
    hasher.update(canonical.as_bytes());
    hasher.update(b"\n");

    let digest = hasher.finalize();
    Ok(hex_encode(&digest))
}

/// Compute SHA-256 signature for a Work Item.
///
/// # Errors
/// Returns an error if serialization fails (should not happen for valid specs).
pub fn compute_work_item_signature(item: &WorkItemEntry) -> Result<String, serde_json::Error> {
    let mut hasher = Sha256::new();

    hasher.update(format!("govctl-signature-v{SIGNATURE_VERSION}\n").as_bytes());
    hasher.update(b"type:work\n");

    let item_json = serde_json::to_value(&item.spec)?;
    let canonical = canonicalize_json(&item_json);
    hasher.update(canonical.as_bytes());
    hasher.update(b"\n");

    let digest = hasher.finalize();
    Ok(hex_encode(&digest))
}

/// Extract signature from rendered markdown content.
///
/// Looks for: `<!-- SIGNATURE: sha256:<hex> -->`
pub fn extract_signature(markdown: &str) -> Option<String> {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("<!-- SIGNATURE: sha256:") {
            if let Some(sig) = rest.strip_suffix(" -->") {
                return Some(sig.trim().to_string());
            }
        }
    }
    None
}

/// Format the signature header comments for embedding in markdown.
pub fn format_signature_header(source_id: &str, signature: &str) -> String {
    format!(
        "<!-- GENERATED: do not edit. Source: {source_id} -->\n\
         <!-- SIGNATURE: sha256:{signature} -->\n"
    )
}

// =============================================================================
// Canonical JSON serialization (deterministic)
// =============================================================================

/// Canonicalize a JSON value:
/// - Object keys sorted alphabetically (recursively)
/// - Arrays preserve order (only objects get key-sorted)
/// - Compact format (no extra whitespace)
fn canonicalize_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical_json(value, &mut out);
    out
}

fn write_canonical_json(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(num) => out.push_str(&num.to_string()),
        Value::String(s) => {
            // Use serde_json's string escaping
            if let Ok(escaped) = serde_json::to_string(s) {
                out.push_str(&escaped);
            }
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical_json(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            // Sort keys alphabetically for determinism
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if let Ok(escaped_key) = serde_json::to_string(*key) {
                    out.push_str(&escaped_key);
                }
                out.push(':');
                write_canonical_json(&map[*key], out);
            }
            out.push('}');
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

/// Check if an RFC has been amended since its last release.
///
/// Returns `true` if the current content signature differs from the stored signature,
/// indicating the RFC has been modified but not yet bumped to a new version.
///
/// Returns `false` if signatures match (clean state) or if no signature is stored (legacy RFC).
pub fn is_rfc_amended(rfc: &RfcIndex) -> bool {
    let Some(stored_sig) = &rfc.rfc.signature else {
        return false; // No signature = legacy RFC, assume clean
    };

    let Ok(current_sig) = compute_rfc_signature(rfc) else {
        return false; // Can't compute = assume clean
    };

    stored_sig != &current_sig
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalize_sorts_keys() {
        let json: Value =
            serde_json::from_str(r#"{"z": 1, "a": 2, "m": 3}"#).expect("test JSON should parse");
        let canonical = canonicalize_json(&json);
        assert_eq!(canonical, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn test_canonicalize_nested_objects() {
        let json: Value = serde_json::from_str(r#"{"outer": {"z": 1, "a": 2}, "inner": {"b": 3}}"#)
            .expect("test JSON should parse");
        let canonical = canonicalize_json(&json);
        assert_eq!(canonical, r#"{"inner":{"b":3},"outer":{"a":2,"z":1}}"#);
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
}
