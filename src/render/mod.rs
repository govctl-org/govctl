//! SSOT to Markdown rendering.
//!
//! Implements [[ADR-0003]] signatures and [[ADR-0011]] inline reference expansion.
//!
//! Rendered markdown files are read-only projections. Each includes:
//! - A "GENERATED" comment warning not to edit
//! - A SHA-256 signature for tampering detection
//! - Inline `[[artifact-id]]` references expanded to markdown links

mod adr;
mod links;
mod output;
mod rfc;
#[cfg(test)]
mod tests;
mod work;

pub use adr::{render_adr, write_adr_md};
pub use links::expand_inline_refs;
use links::render_refs;
use output::write_rendered_md;
pub use rfc::{render_clause, render_rfc, write_rfc};
pub use work::{render_work_item, write_work_item_md};

pub fn ref_link_from_root(ref_id: &str, docs_output: &str) -> String {
    links::ref_link_from_root(ref_id, docs_output)
}

pub fn expand_inline_refs_from_root(text: &str, pattern: &str, docs_output: &str) -> String {
    links::expand_inline_refs_with_linker(text, pattern, |ref_id| {
        ref_link_from_root(ref_id, docs_output)
    })
}
