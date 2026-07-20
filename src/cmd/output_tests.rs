use super::*;
use serde::ser::{Error as _, Serializer};

struct FailingSerialize;

impl Serialize for FailingSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(S::Error::custom("forced failure"))
    }
}

#[test]
fn print_json_maps_serialization_error_to_diagnostic() -> Result<(), String> {
    let result = print_json(
        &FailingSerialize,
        DiagnosticCode::E0903UnexpectedError,
        "Failed to serialize command description",
        "describe",
    );
    let Err(err) = result else {
        return Err("expected serialization failure".to_string());
    };

    assert_eq!(err.code, DiagnosticCode::E0903UnexpectedError);
    assert_eq!(err.file, "describe");
    assert!(
        err.message
            .starts_with("Failed to serialize command description: ")
    );
    assert!(err.message.contains("forced failure"));
    Ok(())
}

#[test]
fn print_toml_maps_serialization_error_to_diagnostic() -> Result<(), String> {
    let result = print_toml(
        &FailingSerialize,
        DiagnosticCode::E1001GuardSchemaInvalid,
        "Failed to serialize guard TOML",
        "GUARD-TEST",
    );
    let Err(err) = result else {
        return Err("expected serialization failure".to_string());
    };

    assert_eq!(err.code, DiagnosticCode::E1001GuardSchemaInvalid);
    assert_eq!(err.file, "GUARD-TEST");
    assert!(err.message.starts_with("Failed to serialize guard TOML: "));
    assert!(err.message.contains("forced failure"));
    Ok(())
}

#[test]
fn print_yaml_maps_serialization_error_to_diagnostic() -> Result<(), String> {
    let result = print_yaml(
        &FailingSerialize,
        DiagnosticCode::E0903UnexpectedError,
        "Failed to serialize YAML",
        "show",
    );
    let Err(err) = result else {
        return Err("expected serialization failure".to_string());
    };

    assert_eq!(err.code, DiagnosticCode::E0903UnexpectedError);
    assert_eq!(err.file, "show");
    assert!(err.message.starts_with("Failed to serialize YAML: "));
    assert!(err.message.contains("forced failure"));
    Ok(())
}
