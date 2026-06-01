use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde::Serialize;

pub(crate) fn print_json_array<T: Serialize>(items: &[T]) {
    println!(
        "{}",
        serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".to_string())
    );
}

pub(crate) fn print_json<T: Serialize>(
    value: &T,
    error_code: DiagnosticCode,
    error_message: &str,
    scope: impl Into<String>,
) -> DiagnosticResult<()> {
    let json = serde_json::to_string_pretty(value).map_err(|err| {
        Diagnostic::new(error_code, format!("{error_message}: {err}"), scope.into())
    })?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
