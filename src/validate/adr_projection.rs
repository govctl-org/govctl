use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrEntry, AlternativeStatus};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

const FIXED_RENDER_HEADINGS: &[&str] = &[
    "Context",
    "Decision",
    "Consequences",
    "References",
    "Alternatives Considered",
    "Options Considered",
];

pub(crate) fn validate_adr_projection_ownership(adr: &AdrEntry, file: String) -> Vec<Diagnostic> {
    let reserved = reserved_headings(adr);
    let fields = [
        ("content.context", adr.spec.content.context.as_str()),
        ("content.decision", adr.spec.content.decision.as_str()),
        (
            "content.consequences",
            adr.spec.content.consequences.as_str(),
        ),
    ];

    let mut diagnostics = Vec::new();
    for (field, markdown) in fields {
        for heading in heading_texts(markdown) {
            if reserved
                .iter()
                .any(|reserved| heading.eq_ignore_ascii_case(reserved))
            {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0307AdrProjectionConflict,
                    format!(
                        "ADR {field} heading '{heading}' conflicts with renderer-owned projection structure"
                    ),
                    file.clone(),
                ));
            }
        }
    }
    diagnostics
}

fn reserved_headings(adr: &AdrEntry) -> Vec<String> {
    let mut headings = FIXED_RENDER_HEADINGS
        .iter()
        .map(|heading| (*heading).to_string())
        .collect::<Vec<_>>();
    headings.extend(heading_texts(&format!(
        "# {}: {}",
        adr.meta().id,
        adr.meta().title
    )));

    for alternative in &adr.spec.content.alternatives {
        let suffix = match alternative.status {
            AlternativeStatus::Considered => "",
            AlternativeStatus::Accepted => " (accepted)",
            AlternativeStatus::Rejected => " (rejected)",
        };
        headings.extend(heading_texts(&format!("# {}{suffix}", alternative.text)));
    }

    headings
}

fn heading_texts(markdown: &str) -> Vec<String> {
    let mut headings = Vec::new();
    let mut current = None;

    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Heading { .. }) => current = Some(String::new()),
            Event::End(TagEnd::Heading(_)) => {
                if let Some(heading) = current.take() {
                    headings.push(heading.trim().to_string());
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = current.as_mut() {
                    heading.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(heading) = current.as_mut() {
                    heading.push(' ');
                }
            }
            _ => {}
        }
    }

    headings
}

#[cfg(test)]
mod tests {
    use super::heading_texts;

    #[test]
    fn extracts_visible_commonmark_heading_text() {
        assert_eq!(
            heading_texts("### **Options** `Considered`\n"),
            vec!["Options Considered"]
        );
        assert_eq!(heading_texts("Decision\n--------\n"), vec!["Decision"]);
    }

    #[test]
    fn ignores_heading_syntax_inside_fenced_code() {
        assert!(heading_texts("```markdown\n## Decision\n```\n").is_empty());
    }
}
