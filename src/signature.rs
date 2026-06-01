//! Deterministic signature computation for rendered projections.
//!
//! Implements [[ADR-0003]] deterministic hash signatures.
//!
//! Signatures ensure rendered markdown files are read-only projections
//! of the authoritative JSON/TOML sources. Any direct edit to the markdown
//! will break the signature, which is detected by `govctl check`.
//!
//! Hash is SHA-256 of canonicalized content with sorted keys.

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrEntry, RfcIndex, WorkItemEntry};
use serde::Serialize;
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
pub fn compute_rfc_signature(rfc: &RfcIndex) -> Result<String, Diagnostic> {
    let mut hasher = signature_hasher("rfc");

    // Canonical RFC metadata (excluding signature field to avoid circularity)
    let mut rfc_json = signature_value(
        &rfc.rfc,
        DiagnosticCode::E0101RfcSchemaInvalid,
        "RFC",
        &rfc.rfc.rfc_id,
    )?;
    if let Value::Object(ref mut map) = rfc_json {
        map.remove("signature"); // Exclude signature from hash computation
    }
    update_canonical_json(&mut hasher, &rfc_json);

    // Sort clauses by clause_id for determinism
    let mut clauses: Vec<_> = rfc.clauses.iter().collect();
    clauses.sort_by(|a, b| a.spec.clause_id.cmp(&b.spec.clause_id));

    // Canonical clause content
    for clause in clauses {
        let clause_json = signature_value(
            &clause.spec,
            DiagnosticCode::E0201ClauseSchemaInvalid,
            "clause",
            format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id),
        )?;
        update_canonical_json(&mut hasher, &clause_json);
    }

    Ok(finalize_signature(hasher))
}

/// Compute SHA-256 signature for an ADR.
///
/// # Errors
/// Returns an error if serialization fails (should not happen for valid specs).
pub fn compute_adr_signature(adr: &AdrEntry) -> Result<String, Diagnostic> {
    compute_simple_signature(
        "adr",
        &adr.spec,
        DiagnosticCode::E0301AdrSchemaInvalid,
        "ADR",
        adr.spec.govctl.id.as_str(),
    )
}

/// Compute SHA-256 signature for a Work Item.
///
/// # Errors
/// Returns an error if serialization fails (should not happen for valid specs).
pub fn compute_work_item_signature(item: &WorkItemEntry) -> Result<String, Diagnostic> {
    compute_simple_signature(
        "work",
        &item.spec,
        DiagnosticCode::E0401WorkSchemaInvalid,
        "work item",
        item.spec.govctl.id.as_str(),
    )
}

fn compute_simple_signature<T: Serialize>(
    kind: &str,
    value: &T,
    code: DiagnosticCode,
    artifact: &str,
    id: impl Into<String>,
) -> Result<String, Diagnostic> {
    let mut hasher = signature_hasher(kind);
    let value_json = signature_value(value, code, artifact, id)?;
    update_canonical_json(&mut hasher, &value_json);

    Ok(finalize_signature(hasher))
}

fn signature_hasher(kind: &str) -> Sha256 {
    let mut hasher = Sha256::new();
    hasher.update(format!("govctl-signature-v{SIGNATURE_VERSION}\n").as_bytes());
    hasher.update(format!("type:{kind}\n").as_bytes());
    hasher
}

fn signature_value<T: Serialize>(
    value: &T,
    code: DiagnosticCode,
    artifact: &str,
    id: impl Into<String>,
) -> Result<Value, Diagnostic> {
    serde_json::to_value(value).map_err(|err| {
        Diagnostic::new(
            code,
            format!("Failed to serialize {artifact} for signature: {err}"),
            id,
        )
    })
}

fn update_canonical_json(hasher: &mut Sha256, value: &Value) {
    let canonical = canonicalize_json(value);
    hasher.update(canonical.as_bytes());
    hasher.update(b"\n");
}

fn finalize_signature(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    hex_encode(&digest)
}

/// Extract signature from rendered markdown content.
///
/// Looks for: `<!-- SIGNATURE: sha256:<hex> -->`
pub fn extract_signature(markdown: &str) -> Option<String> {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("<!-- SIGNATURE: sha256:")
            && let Some(sig) = rest.strip_suffix(" -->")
        {
            return Some(sig.trim().to_string());
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
    fn test_canonicalize_sorts_keys() -> Result<(), Box<dyn std::error::Error>> {
        let json: Value = serde_json::from_str(r#"{"z": 1, "a": 2, "m": 3}"#)?;
        let canonical = canonicalize_json(&json);
        assert_eq!(canonical, r#"{"a":2,"m":3,"z":1}"#);
        Ok(())
    }

    #[test]
    fn test_canonicalize_nested_objects() -> Result<(), Box<dyn std::error::Error>> {
        let json: Value =
            serde_json::from_str(r#"{"outer": {"z": 1, "a": 2}, "inner": {"b": 3}}"#)?;
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
}
