//! Shared validation for configured inline artifact-reference patterns.

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use regex::{Captures, Match, Regex};
use std::fmt;

pub(crate) fn compile(pattern: &str, config_path: impl Into<String>) -> DiagnosticResult<Regex> {
    let config_path = config_path.into();
    let regex = Regex::new(pattern).map_err(|error| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            format!("Invalid source_scan.pattern regex: {error}"),
            config_path.clone(),
        )
    })?;

    if regex.captures_len() < 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            "Invalid source_scan.pattern: capture group 1 is required",
            config_path,
        ));
    }

    Ok(regex)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum TargetCaptureError {
    Absent,
    Empty,
}

impl fmt::Display for TargetCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => write!(
                formatter,
                "capture group 1 did not participate in the match"
            ),
            Self::Empty => write!(formatter, "capture group 1 matched an empty target"),
        }
    }
}

pub(crate) fn target_capture<'text>(
    captures: &Captures<'text>,
) -> Result<Match<'text>, TargetCaptureError> {
    let target = captures.get(1).ok_or(TargetCaptureError::Absent)?;
    if target.is_empty() {
        return Err(TargetCaptureError::Empty);
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_requires_capture_group_one() -> DiagnosticResult<()> {
        let Err(diagnostic) = compile(r"\[\[RFC-\d{4}\]\]", "config") else {
            return Err(Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                "pattern without capture group 1 compiled",
                "test",
            ));
        };

        assert_eq!(diagnostic.code, DiagnosticCode::E0501ConfigInvalid);
        assert!(diagnostic.message.contains("capture group 1 is required"));
        Ok(())
    }

    #[test]
    fn target_capture_rejects_absent_and_empty_matches() -> DiagnosticResult<()> {
        let optional = compile(r"(?:plain|\[\[([^]]+)\]\])", "config")?;
        let absent = optional.captures("plain").ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                "optional-capture test pattern did not match",
                "test",
            )
        })?;
        assert_eq!(target_capture(&absent), Err(TargetCaptureError::Absent));

        let empty = compile(r"\[\[([^]]*)\]\]", "config")?;
        let empty_capture = empty.captures("[[]]").ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                "empty-capture test pattern did not match",
                "test",
            )
        })?;
        assert_eq!(
            target_capture(&empty_capture),
            Err(TargetCaptureError::Empty)
        );
        Ok(())
    }
}
