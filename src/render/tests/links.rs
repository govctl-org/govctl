use super::*;

#[test]
fn test_expand_inline_refs_rfc() {
    let text = "See [[RFC-0000]] for details.";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(result, "See [RFC-0000](../rfc/RFC-0000.md) for details.");
}

#[test]
fn test_expand_inline_refs_clause() {
    let text = "Per [[RFC-0000:C-WORK-DEF]], work items must...";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        "Per [RFC-0000:C-WORK-DEF](../rfc/RFC-0000.md#rfc-0000c-work-def), work items must..."
    );
}

#[test]
fn test_expand_inline_refs_adr() {
    let text = "This follows [[ADR-0005]] guidelines.";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        "This follows [ADR-0005](../adr/ADR-0005.md) guidelines."
    );
}

#[test]
fn test_expand_inline_refs_multiple() {
    let text = "See [[RFC-0000]] and [[ADR-0042]] for context.";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        "See [RFC-0000](../rfc/RFC-0000.md) and [ADR-0042](../adr/ADR-0042.md) for context."
    );
}

#[test]
fn test_expand_inline_refs_no_match() {
    let text = "No references here.";
    let result = expand_inline_refs(text, DEFAULT_PATTERN);
    assert_eq!(result, "No references here.");
}

#[test]
fn test_expand_inline_refs_invalid_pattern() {
    let text = "[[RFC-0000]] test";
    let result = expand_inline_refs(text, "[invalid(regex");
    assert_eq!(result, "[[RFC-0000]] test");
}

#[test]
fn test_ref_link_from_root_rfc() {
    let result = ref_link_from_root("RFC-0000", "docs");
    assert_eq!(result, "[RFC-0000](docs/rfc/RFC-0000.md)");
}

#[test]
fn test_ref_link_from_root_clause() {
    let result = ref_link_from_root("RFC-0000:C-WORK-DEF", "docs");
    assert_eq!(
        result,
        "[RFC-0000:C-WORK-DEF](docs/rfc/RFC-0000.md#rfc-0000c-work-def)"
    );
}

#[test]
fn test_ref_link_from_root_adr() {
    let result = ref_link_from_root("ADR-0005", "docs");
    assert_eq!(result, "[ADR-0005](docs/adr/ADR-0005.md)");
}

#[test]
fn test_ref_link_from_root_custom_path() {
    let result = ref_link_from_root("RFC-0001", "documentation");
    assert_eq!(result, "[RFC-0001](documentation/rfc/RFC-0001.md)");
}

#[test]
fn test_expand_inline_refs_from_root() {
    let text = "Per [[RFC-0002:C-RESOURCE-MODEL]], resources use verb pattern.";
    let result = expand_inline_refs_from_root(text, DEFAULT_PATTERN, "docs");
    assert_eq!(
        result,
        "Per [RFC-0002:C-RESOURCE-MODEL](docs/rfc/RFC-0002.md#rfc-0002c-resource-model), resources use verb pattern."
    );
}

#[test]
fn test_expand_inline_refs_from_root_multiple() {
    let text = "See [[RFC-0000]] and [[ADR-0018]] for details.";
    let result = expand_inline_refs_from_root(text, DEFAULT_PATTERN, "docs");
    assert_eq!(
        result,
        "See [RFC-0000](docs/rfc/RFC-0000.md) and [ADR-0018](docs/adr/ADR-0018.md) for details."
    );
}

#[test]
fn test_expand_inline_refs_work_item_sequential() {
    let id = "WI-9999-01-26-001";
    let text = format!("See {} for task details.", wi_ref(id));
    let result = expand_inline_refs(&text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        format!("See [{}](../work/{}.md) for task details.", id, id)
    );
}

#[test]
fn test_expand_inline_refs_work_item_author_hash() {
    let id = "WI-9999-01-26-a7f3-001";
    let text = format!("See {} for task details.", wi_ref(id));
    let result = expand_inline_refs(&text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        format!("See [{}](../work/{}.md) for task details.", id, id)
    );
}

#[test]
fn test_expand_inline_refs_work_item_random() {
    let id = "WI-9999-01-26-b2c9";
    let text = format!("See {} for task details.", wi_ref(id));
    let result = expand_inline_refs(&text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        format!("See [{}](../work/{}.md) for task details.", id, id)
    );
}

#[test]
fn test_expand_inline_refs_work_item_mixed() {
    let wi_id = "WI-9999-01-26-001";
    let text = format!("Per [[RFC-0000]], see {} and [[ADR-0020]].", wi_ref(wi_id));
    let result = expand_inline_refs(&text, DEFAULT_PATTERN);
    assert_eq!(
        result,
        format!(
            "Per [RFC-0000](../rfc/RFC-0000.md), see [{}](../work/{}.md) and [ADR-0020](../adr/ADR-0020.md).",
            wi_id, wi_id
        )
    );
}
