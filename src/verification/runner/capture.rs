use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::GuardEntry;
use std::io::{Read, Seek, SeekFrom};
use std::process::Stdio;
use tempfile::NamedTempFile;

pub(super) struct GuardOutputCapture {
    file: NamedTempFile,
}

impl GuardOutputCapture {
    pub(super) fn new(guard: &GuardEntry, stream_name: &str) -> Result<Self, Diagnostic> {
        let file = NamedTempFile::new().map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to create {stream_name} capture file for verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })?;
        Ok(Self { file })
    }

    pub(super) fn stdio(&self, guard: &GuardEntry, stream_name: &str) -> Result<Stdio, Diagnostic> {
        self.file.reopen().map(Stdio::from).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to prepare {stream_name} capture file for verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })
    }

    pub(super) fn read(
        &mut self,
        guard: &GuardEntry,
        stream_name: &str,
    ) -> Result<Vec<u8>, Diagnostic> {
        let file = self.file.as_file_mut();
        file.seek(SeekFrom::Start(0)).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to rewind {stream_name} capture file for verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })?;

        let mut output = Vec::new();
        file.read_to_end(&mut output).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E1004GuardCheckFailed,
                format!(
                    "Failed to collect {stream_name} for verification guard '{}': {}",
                    guard.meta().id,
                    err
                ),
                guard.path.display().to_string(),
            )
        })?;
        Ok(output)
    }
}
