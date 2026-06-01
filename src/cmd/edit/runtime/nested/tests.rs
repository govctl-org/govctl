use super::*;
use serde_json::json;

fn path(input: &str) -> Result<FieldPath, Box<dyn std::error::Error>> {
    Ok(crate::cmd::edit::path::parse_field_path(input)?.collapse_legacy_prefixes())
}

#[test]
fn test_add_nested_object_list_value_deduplicates_by_text() -> Result<(), Box<dyn std::error::Error>>
{
    let mut doc = json!({
        "content": {
            "alternatives": [
                { "text": "Option A", "status": "considered", "pros": [], "cons": [] }
            ]
        }
    });

    add_nested_list_value(
        ArtifactType::Adr,
        &mut doc,
        &path("alternatives")?,
        "Option A",
        "ADR-0001",
    )?;
    add_nested_list_value(
        ArtifactType::Adr,
        &mut doc,
        &path("alternatives")?,
        "Option B",
        "ADR-0001",
    )?;

    let alternatives = doc["content"]["alternatives"]
        .as_array()
        .ok_or("expected array")?;
    assert_eq!(alternatives.len(), 2);
    assert_eq!(alternatives[1]["text"], "Option B");
    Ok(())
}

#[test]
fn test_set_nested_field_rejects_list_path_without_index() -> Result<(), Box<dyn std::error::Error>>
{
    let mut doc = json!({
        "content": {
            "alternatives": [
                { "text": "Option A", "status": "considered", "pros": [], "cons": [] }
            ]
        }
    });

    let result = set_nested_field(
        ArtifactType::Adr,
        &mut doc,
        &path("alternatives[0].pros")?,
        "oops",
        "ADR-0001",
    );
    assert!(result.is_err());
    let err = result.err().ok_or("expected Err")?;
    let diag = err
        .downcast_ref::<Diagnostic>()
        .ok_or("expected Diagnostic")?;
    assert_eq!(diag.code, DiagnosticCode::E0817PathTypeMismatch);
    Ok(())
}

#[test]
fn test_get_nested_field_renders_object_item_with_scalar_lists()
-> Result<(), Box<dyn std::error::Error>> {
    let doc = json!({
        "content": {
            "alternatives": [
                {
                    "text": "Option A",
                    "status": "accepted",
                    "pros": ["Readable", "Simple"],
                    "cons": ["More maintenance"],
                    "rejection_reason": null
                }
            ]
        }
    });

    let rendered = get_nested_field(
        ArtifactType::Adr,
        &doc,
        &path("alternatives[0]")?,
        "ADR-0001",
    )?;

    assert!(rendered.contains("text: Option A"));
    assert!(rendered.contains("status: accepted"));
    assert!(rendered.contains("pros: Readable, Simple"));
    assert!(rendered.contains("cons: More maintenance"));
    Ok(())
}
