//! Project support file synchronization shared by init, migrate, and check.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::ui;
use crate::write::{WriteOp, write_file};
use std::io::ErrorKind;
use std::path::PathBuf;

// Implements [[RFC-0002:C-GLOBAL-COMMANDS]]: init/migrate maintain local-state ignore entries.
const LOCAL_STATE_GITIGNORE_ENTRIES: &[&str] = &[".govctl.lock", ".govctl/"];

// Implements [[RFC-0002:C-GLOBAL-COMMANDS]]: migrate refreshes local-state
// .gitignore entries regardless of schema version.
pub(crate) fn ensure_local_state_gitignore_entries(
    config: &Config,
    op: WriteOp,
) -> DiagnosticResult<usize> {
    let gitignore_path = gitignore_path(config);
    let display_path = config.display_path(&gitignore_path);

    match std::fs::read_to_string(&gitignore_path) {
        Ok(content) => {
            let missing_entries = missing_local_state_gitignore_entries(&content);
            if missing_entries.is_empty() {
                return Ok(0);
            }

            let missing_content = missing_entries.join("\n");
            let new_content = if content.ends_with('\n') {
                format!("{content}{missing_content}\n")
            } else {
                format!("{content}\n{missing_content}\n")
            };
            write_file(&gitignore_path, &new_content, op, Some(&display_path))?;
            if !op.is_preview() {
                ui::info(format!(
                    "Added local govctl state entries to .gitignore: {}",
                    missing_entries.join(", ")
                ));
            }
            Ok(missing_entries.len())
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let content = format!(
                "# govctl local state\n{}\n",
                LOCAL_STATE_GITIGNORE_ENTRIES.join("\n")
            );
            write_file(&gitignore_path, &content, op, Some(&display_path))?;
            if !op.is_preview() {
                ui::created_path(&display_path);
            }
            Ok(LOCAL_STATE_GITIGNORE_ENTRIES.len())
        }
        Err(err) => Err(Diagnostic::io_error(
            "read .gitignore",
            err,
            display_path.display().to_string(),
        )),
    }
}

// Implements [[RFC-0002:C-GLOBAL-COMMANDS]]: check warns when govctl-managed
// local-state .gitignore entries are missing or outdated.
pub(crate) fn local_state_gitignore_diagnostics(config: &Config) -> Diagnostics {
    let gitignore_path = gitignore_path(config);
    let display_path = config.display_path(&gitignore_path);
    let missing_entries = match std::fs::read_to_string(&gitignore_path) {
        Ok(content) => missing_local_state_gitignore_entries(&content),
        Err(err) if err.kind() == ErrorKind::NotFound => LOCAL_STATE_GITIGNORE_ENTRIES.to_vec(),
        Err(err) => {
            return vec![Diagnostic::io_error(
                "read .gitignore",
                err,
                display_path.display().to_string(),
            )];
        }
    };

    if missing_entries.is_empty() {
        return vec![];
    }

    vec![Diagnostic::new(
        DiagnosticCode::W0111ProjectSupportOutdated,
        format!(
            "Local govctl state entries missing from .gitignore: {}. Run `govctl migrate` to refresh local-state .gitignore entries.",
            missing_entries.join(", ")
        ),
        display_path.display().to_string(),
    )]
}

fn gitignore_path(config: &Config) -> PathBuf {
    config.project_root().join(".gitignore")
}

fn missing_local_state_gitignore_entries(content: &str) -> Vec<&'static str> {
    LOCAL_STATE_GITIGNORE_ENTRIES
        .iter()
        .copied()
        .filter(|entry| !content.lines().any(|line| line.trim() == *entry))
        .collect()
}
