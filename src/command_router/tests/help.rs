use super::*;

#[test]
fn test_rfc_get_help_restores_resource_specific_examples() {
    let err = match crate::Cli::try_parse_from(["govctl", "rfc", "get", "--help"]) {
        Ok(_) => unreachable!("help should exit"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    let help = err.to_string();
    assert!(help.contains("VALID FIELDS:"), "help: {help}");
    assert!(
        help.contains("govctl rfc get RFC-0001 title"),
        "help: {help}"
    );
}

#[test]
fn test_work_get_help_restores_resource_specific_examples() {
    let err = match crate::Cli::try_parse_from(["govctl", "work", "get", "--help"]) {
        Ok(_) => unreachable!("help should exit"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    let help = err.to_string();
    assert!(help.contains("VALID FIELDS:"), "help: {help}");
    assert!(
        help.contains("acceptance_criteria[0].status"),
        "help: {help}"
    );
}

#[test]
fn test_release_help_exposes_creation_and_undo() {
    let err = match crate::Cli::try_parse_from(["govctl", "release", "--help"]) {
        Ok(_) => unreachable!("help should exit"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    let help = err.to_string();
    assert!(help.contains("[VERSION]"), "help: {help}");
    assert!(help.contains("undo"), "help: {help}");
    assert!(help.contains("govctl release 0.2.0"), "help: {help}");
    assert!(help.contains("govctl release undo 0.2.0"), "help: {help}");
}
