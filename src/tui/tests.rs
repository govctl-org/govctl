use super::*;

#[test]
fn test_project_load_error_prefers_error_diagnostic() {
    let diag = project_load_error(
        vec![
            Diagnostic::new(DiagnosticCode::W0109WorkNoActive, "warning", "warn"),
            Diagnostic::new(DiagnosticCode::E0302AdrNotFound, "missing adr", "gov/adr"),
        ],
        std::path::Path::new("gov"),
    );

    assert_eq!(diag.code, DiagnosticCode::E0302AdrNotFound);
    assert_eq!(diag.message, "missing adr");
}

#[test]
fn test_project_load_error_uses_warning_when_no_errors_exist() {
    let diag = project_load_error(
        vec![Diagnostic::new(
            DiagnosticCode::W0109WorkNoActive,
            "warning only",
            "gov/work",
        )],
        std::path::Path::new("gov"),
    );

    assert_eq!(diag.code, DiagnosticCode::W0109WorkNoActive);
    assert_eq!(diag.message, "warning only");
}

#[test]
fn test_project_load_error_falls_back_when_empty() {
    let diag = project_load_error(vec![], std::path::Path::new("gov"));

    assert_eq!(diag.code, DiagnosticCode::E0501ConfigInvalid);
    assert_eq!(diag.message, "Failed to load project");
    assert_eq!(diag.file, "gov");
}
