//! Styled terminal markdown rendering.
//!
//! Transforms raw markdown (generated for file output) into styled
//! ANSI terminal output. Strips HTML artifacts, converts checkboxes,
//! and renders through markdown-to-ansi when the terminal supports it.

use crate::ui::stdout_supports_color;
use regex::Regex;
use std::sync::LazyLock;

static HTML_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<!--[\s\S]*?-->").expect("valid regex"));

static HTML_ANCHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\s*<a\s+id="[^"]*"\s*></a>"#).expect("valid regex"));

static HTML_DEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"</?del>").expect("valid regex"));

static MD_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\([^)]+\.md(?:#[^)]*)?\)").expect("valid regex"));

/// Strip HTML artifacts that are meaningful in rendered .md files
/// but visual noise in a terminal.
pub fn strip_for_terminal(md: &str) -> String {
    let s = HTML_COMMENT.replace_all(md, "");
    let s = HTML_ANCHOR.replace_all(&s, "");
    let s = HTML_DEL.replace_all(&s, "");
    let s = MD_LINK.replace_all(&s, "$1");

    let mut out = String::with_capacity(s.len());
    for line in s.lines() {
        let transformed = if let Some(rest) = line.strip_prefix("- [x] ") {
            format!("- \u{2713} {rest}")
        } else if let Some(rest) = line.strip_prefix("- [ ] ") {
            format!("- \u{25CB} {rest}")
        } else {
            line.to_string()
        };
        out.push_str(&transformed);
        out.push('\n');
    }

    // Collapse runs of 3+ blank lines into 2
    while out.contains("\n\n\n\n") {
        out = out.replace("\n\n\n\n", "\n\n\n");
    }

    out
}

/// Render markdown with ANSI styling for terminal display.
///
/// When stdout is a TTY and NO_COLOR is not set, renders styled output
/// via markdown-to-ansi. Otherwise returns cleaned plain markdown.
pub fn render_terminal_md(md: &str) -> String {
    let stripped = strip_for_terminal(md);
    let clean = stripped.trim();

    if clean.is_empty() {
        return String::new();
    }

    if !stdout_supports_color() {
        return clean.to_string();
    }

    let width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80);

    let opts = markdown_to_ansi::Options {
        syntax_highlight: true,
        width: Some(width),
        code_bg: true,
    };

    markdown_to_ansi::render(clean, &opts)
}

/// Render markdown to a ratatui `Text` widget via the shared pipeline.
///
/// markdown → strip_for_terminal → markdown-to-ansi → ansi-to-tui → Text
#[cfg(feature = "tui")]
pub fn render_to_tui_text(md: &str) -> ratatui::text::Text<'static> {
    use ansi_to_tui::IntoText;

    let stripped = strip_for_terminal(md);
    let clean = stripped.trim();
    if clean.is_empty() {
        return ratatui::text::Text::default();
    }

    let width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80);

    let opts = markdown_to_ansi::Options {
        syntax_highlight: true,
        width: Some(width),
        code_bg: false,
    };

    let ansi = markdown_to_ansi::render(clean, &opts);
    ansi.into_text().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_comments() {
        let input = "<!-- GENERATED: do not edit -->\n<!-- SIGNATURE: sha256:abc -->\n\n# Title\n";
        let result = strip_for_terminal(input);
        assert!(!result.contains("<!--"));
        assert!(result.contains("# Title"));
    }

    #[test]
    fn test_strip_anchors() {
        let input = "### [RFC-0003:C-NAV] Title (Normative) <a id=\"rfc-0003c-nav\"></a>\n";
        let result = strip_for_terminal(input);
        assert!(!result.contains("<a id"));
        assert!(result.contains("[RFC-0003:C-NAV] Title (Normative)"));
    }

    #[test]
    fn test_strip_del_tags() {
        let input = "### <del>[RFC-0000:C-OLD] Deprecated Clause</del> (Normative)\n";
        let result = strip_for_terminal(input);
        assert!(!result.contains("<del>"));
        assert!(!result.contains("</del>"));
        assert!(result.contains("[RFC-0000:C-OLD] Deprecated Clause"));
    }

    #[test]
    fn test_convert_relative_links() {
        let input = "**References:** [RFC-0000](../rfc/RFC-0000.md)\n";
        let result = strip_for_terminal(input);
        assert!(result.contains("RFC-0000"));
        assert!(!result.contains("../rfc/RFC-0000.md"));
    }

    #[test]
    fn test_convert_clause_links() {
        let input = "See [RFC-0001:C-FOO](../rfc/RFC-0001.md#rfc-0001c-foo) for details.\n";
        let result = strip_for_terminal(input);
        assert!(result.contains("RFC-0001:C-FOO"));
        assert!(!result.contains(".md#"));
    }

    #[test]
    fn test_transform_checkboxes() {
        let input = "- [x] Done item\n- [ ] Pending item\n- Regular item\n";
        let result = strip_for_terminal(input);
        assert!(result.contains("- \u{2713} Done item"));
        assert!(result.contains("- \u{25CB} Pending item"));
        assert!(result.contains("- Regular item"));
    }

    #[test]
    fn test_no_color_returns_plain() {
        // SAFETY: test runs single-threaded; no concurrent env access.
        unsafe { std::env::set_var("NO_COLOR", "1") };
        let md = "# Hello\n\nSome **bold** text.\n";
        let result = render_terminal_md(md);
        assert!(!result.contains("\x1b["));
        unsafe { std::env::remove_var("NO_COLOR") };
    }
}
