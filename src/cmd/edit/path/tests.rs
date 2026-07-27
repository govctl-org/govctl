use super::*;

#[test]
fn test_simple_field() -> Result<(), Box<dyn std::error::Error>> {
    let p = parse_field_path("title")?;
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].name, "title");
    assert_eq!(p.segments[0].index, None);
    assert!(p.is_simple());
    assert_eq!(p.as_simple(), Some("title"));
    Ok(())
}

#[test]
fn test_indexed_field() -> Result<(), Box<dyn std::error::Error>> {
    let p = parse_field_path("alternatives[0]")?;
    assert_eq!(p.segments.len(), 1);
    assert_eq!(p.segments[0].name, "alternatives");
    assert_eq!(p.segments[0].index, Some(0));
    assert!(!p.is_simple());
    assert!(p.has_terminal_index());
    Ok(())
}

#[test]
fn test_dotted_path() -> Result<(), Box<dyn std::error::Error>> {
    let p = parse_field_path("alternatives[0].pros")?;
    assert_eq!(p.segments.len(), 2);
    assert_eq!(p.segments[0].name, "alternatives");
    assert_eq!(p.segments[0].index, Some(0));
    assert_eq!(p.segments[1].name, "pros");
    assert_eq!(p.segments[1].index, None);
    assert!(!p.is_simple());
    assert!(!p.has_terminal_index());
    Ok(())
}

#[test]
fn test_dotted_path_with_terminal_index() -> Result<(), Box<dyn std::error::Error>> {
    let p = parse_field_path("alternatives[0].pros[1]")?;
    assert_eq!(p.segments.len(), 2);
    assert_eq!(p.segments[0].name, "alternatives");
    assert_eq!(p.segments[0].index, Some(0));
    assert_eq!(p.segments[1].name, "pros");
    assert_eq!(p.segments[1].index, Some(1));
    assert!(p.has_terminal_index());
    Ok(())
}

#[test]
fn test_negative_index() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parse_field_path("alternatives[-1]").is_err());
    Ok(())
}

#[test]
fn test_parser_preserves_noncanonical_names() -> Result<(), Box<dyn std::error::Error>> {
    let p = parse_field_path("alt[0]")?;
    assert_eq!(p.segments[0].name, "alt");
    Ok(())
}

#[test]
fn test_raw_parse_keeps_alias_token() -> Result<(), Box<dyn std::error::Error>> {
    let p = parse_raw_field_path("alt[0]")?;
    assert_eq!(p.segments[0].name, "alt");
    Ok(())
}

#[test]
fn test_empty_path() {
    assert!(parse_field_path("").is_err());
}

#[test]
fn test_invalid_start_char() {
    assert!(parse_field_path("0invalid").is_err());
    assert!(parse_field_path("[0]").is_err());
    assert!(parse_field_path("Alt").is_err());
}

#[test]
fn test_double_index() {
    assert!(parse_field_path("alternatives[0][1]").is_err());
}

#[test]
fn test_full_consumption_rejects_trailing_garbage() {
    assert!(parse_field_path("alternatives[0]oops").is_err());
}

#[test]
fn test_empty_index() {
    assert!(parse_field_path("alternatives[]").is_err());
}

#[test]
fn test_unclosed_bracket() {
    assert!(parse_field_path("alternatives[0").is_err());
}

#[test]
fn test_trailing_dot() {
    assert!(parse_field_path("alternatives.").is_err());
}

#[test]
fn test_resolve_index_zero() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(resolve_index(0, 3)?, 0);
    Ok(())
}

#[test]
fn test_resolve_index_positive() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(resolve_index(2, 5)?, 2);
    Ok(())
}

#[test]
fn test_resolve_index_negative_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    assert!(resolve_index(-1, 3).is_err());
    Ok(())
}

#[test]
fn test_resolve_index_out_of_bounds() {
    assert!(resolve_index(3, 3).is_err());
    assert!(resolve_index(-4, 3).is_err());
}

#[test]
fn test_resolve_index_empty_array() {
    assert!(resolve_index(0, 0).is_err());
}
