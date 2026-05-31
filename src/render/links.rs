use regex::Regex;

/// Generate a markdown link for an artifact reference.
///
/// Supports:
/// - RFC refs: `RFC-0000` -> `[RFC-0000](../rfc/RFC-0000.md)`
/// - Clause refs: `RFC-0000:C-NAME` -> `[RFC-0000:C-NAME](../rfc/RFC-0000.md#rfc-0000c-name)`
/// - ADR refs: `ADR-0042` -> `[ADR-0042](../adr/ADR-0042.md)`
/// - Work Item refs: `WI-2026-01-17-001` -> `[WI-2026-01-17-001](../work/WI-2026-01-17-001.md)`
fn ref_link(ref_id: &str) -> String {
    ref_link_with_base(ref_id, "..")
}

/// Generate a markdown link for an artifact reference from the repository root.
///
/// Used for files like CHANGELOG.md that live at the root level.
/// The `docs_output` path comes from config, for example "docs".
pub fn ref_link_from_root(ref_id: &str, docs_output: &str) -> String {
    ref_link_with_base(ref_id, docs_output)
}

/// Generate a markdown link with a configurable base path.
///
/// `base` is the path prefix before `/rfc/`, `/adr/`, `/work/`, for example ".." or "docs".
fn ref_link_with_base(ref_id: &str, base: &str) -> String {
    if ref_id.starts_with("RFC-") {
        if ref_id.contains(':') {
            let rfc_id = ref_id.split(':').next().unwrap_or(ref_id);
            let anchor = ref_id.to_lowercase().replace(':', "");
            format!("[{}]({}/rfc/{}.md#{})", ref_id, base, rfc_id, anchor)
        } else {
            format!("[{}]({}/rfc/{}.md)", ref_id, base, ref_id)
        }
    } else if ref_id.starts_with("ADR-") {
        format!("[{}]({}/adr/{}.md)", ref_id, base, ref_id)
    } else if ref_id.starts_with("WI-") {
        format!("[{}]({}/work/{}.md)", ref_id, base, ref_id)
    } else {
        ref_id.to_string()
    }
}

/// Render a list of refs as markdown links.
pub(super) fn render_refs(refs: &[String]) -> String {
    refs.iter()
        .map(|r| ref_link(r))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Expand inline `[[artifact-id]]` references to markdown links.
///
/// Uses the pattern from source_scan config. The pattern must have a capture group for the
/// artifact ID.
pub fn expand_inline_refs(text: &str, pattern: &str) -> String {
    expand_inline_refs_with_linker(text, pattern, ref_link)
}

pub(super) fn expand_inline_refs_with_linker<F>(text: &str, pattern: &str, linker: F) -> String
where
    F: Fn(&str) -> String,
{
    let Ok(re) = Regex::new(pattern) else {
        return text.to_string();
    };

    re.replace_all(text, |caps: &regex::Captures| {
        if let Some(artifact_id) = caps.get(1) {
            linker(artifact_id.as_str())
        } else {
            caps.get(0).map_or("", |m| m.as_str()).to_string()
        }
    })
    .to_string()
}
