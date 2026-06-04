use super::ArtifactType;
use crate::diagnostic::DiagnosticCode;

#[derive(Clone, Copy)]
pub(super) enum RenderMode {
    Scalar,
    CsvStrings,
    LineStrings,
    StatusLines {
        status_key: &'static str,
        text_key: &'static str,
    },
}

#[derive(Clone, Copy)]
pub(super) struct SimpleFieldSpec {
    pub path: &'static [&'static str],
    pub render: RenderMode,
}

#[derive(Clone, Copy)]
pub(super) enum SetMode {
    String,
    Integer,
    Enum {
        allowed: &'static [&'static str],
        invalid_msg: &'static str,
        code: Option<DiagnosticCode>,
    },
}

#[derive(Clone, Copy)]
pub(super) struct SimpleSetSpec {
    pub path: &'static [&'static str],
    pub mode: SetMode,
}

#[derive(Clone, Copy)]
pub(super) struct StatusListSpec {
    pub path: &'static [&'static str],
    pub status_key: &'static str,
    pub text_key: &'static str,
}

#[derive(Clone, Copy)]
pub(super) struct RuntimeFieldEntry {
    pub artifact: ArtifactType,
    pub field: &'static str,
    pub get: Option<SimpleFieldSpec>,
    pub set: Option<SimpleSetSpec>,
    pub list_path: Option<&'static [&'static str]>,
}
