//! Deterministic signature computation for rendered projections.
//!
//! Implements [[ADR-0003]] deterministic hash signatures.
//!
//! Signatures ensure rendered markdown files are read-only projections
//! of the authoritative JSON/TOML sources. Any direct edit to the markdown
//! will break the signature, which is detected by `govctl check`.

mod canonical_json;
#[cfg(test)]
mod tests;

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrEntry, RfcIndex, WorkItemEntry};
use canonical_json::canonicalize_json;
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
/// Returns a diagnostic if an RFC or clause cannot be serialized for signature input.
pub fn compute_rfc_signature(rfc: &RfcIndex) -> Result<String, Diagnostic> {
    let mut hasher = signature_hasher("rfc");

    let mut rfc_json = signature_value(
        &rfc.rfc,
        DiagnosticCode::E0101RfcSchemaInvalid,
        "RFC",
        &rfc.rfc.rfc_id,
    )?;
    if let Value::Object(ref mut map) = rfc_json {
        map.remove("signature");
    }
    update_canonical_json(&mut hasher, &rfc_json);

    let mut clauses: Vec<_> = rfc.clauses.iter().collect();
    clauses.sort_by(|a, b| a.spec.clause_id.cmp(&b.spec.clause_id));

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
/// Returns a diagnostic if the ADR cannot be serialized for signature input.
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
/// Returns a diagnostic if the work item cannot be serialized for signature input.
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

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push(HEX[(byte >> 4) as usize] as char);
        hex.push(HEX[(byte & 0x0f) as usize] as char);
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
        return false;
    };

    let Ok(current_sig) = compute_rfc_signature(rfc) else {
        return false;
    };

    stored_sig != &current_sig
}
