//! Self-update command: download and replace the govctl binary from GitHub Releases.
//!
//! Implements [[RFC-0002:C-SELF-UPDATE]].

use std::io::IsTerminal;

use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::ui;

const REPO_OWNER: &str = "govctl-org";
const REPO_NAME: &str = "govctl";
const BIN_NAME: &str = "govctl";
const SELF_UPDATE_BIN_PATH_IN_ARCHIVE: &str = "govctl-v{{ version }}-{{ target }}/{{ bin }}";

/// Result of comparing the current version against the latest available.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum VersionCheck {
    /// Current version is up to date (latest <= current).
    UpToDate,
    /// A newer version is available.
    UpdateAvailable { current: String, latest: String },
}

/// Compare two semver version strings. Returns whether an update is available.
pub(crate) fn compare_versions(current: &str, latest_raw: &str) -> DiagnosticResult<VersionCheck> {
    let latest = latest_raw.trim_start_matches('v');

    let current_semver = semver::Version::parse(current).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("failed to parse current version '{current}': {e}"),
            "",
        )
    })?;
    let latest_semver = semver::Version::parse(latest).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("failed to parse latest version '{latest}': {e}"),
            "",
        )
    })?;

    if latest_semver <= current_semver {
        Ok(VersionCheck::UpToDate)
    } else {
        Ok(VersionCheck::UpdateAvailable {
            current: current.to_string(),
            latest: latest.to_string(),
        })
    }
}

/// Check for the latest version and optionally update the binary.
pub fn self_update(check_only: bool) -> DiagnosticResult<Diagnostics> {
    let current = env!("CARGO_PKG_VERSION");

    if check_only {
        check_version(current)
    } else {
        perform_update(current)
    }
}

fn check_version(current: &str) -> DiagnosticResult<Diagnostics> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .map_err(|err| self_update_error("configure GitHub release check", err))?
        .fetch()
        .map_err(|err| self_update_error("fetch GitHub releases", err))?;

    let latest = releases.first().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "no releases found on GitHub",
            "",
        )
    })?;

    match compare_versions(current, &latest.version)? {
        VersionCheck::UpToDate => {
            ui::success(format!("govctl v{current} is up to date"));
            Ok(vec![])
        }
        VersionCheck::UpdateAvailable {
            current: cur,
            latest: lat,
        } => {
            ui::info(format!("govctl v{cur} -> v{lat} available"));
            // Per [[RFC-0002:C-SELF-UPDATE]]: --check MUST exit 1 if a newer version is available.
            // Return an error-level diagnostic so main.rs produces ExitCode::FAILURE.
            Ok(vec![Diagnostic::new(
                DiagnosticCode::E0901IoError,
                format!("update available: v{cur} -> v{lat}"),
                String::new(),
            )])
        }
    }
}

fn perform_update(current: &str) -> DiagnosticResult<Diagnostics> {
    let show_progress = std::io::stdout().is_terminal();

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .bin_path_in_archive(SELF_UPDATE_BIN_PATH_IN_ARCHIVE)
        .show_download_progress(show_progress)
        .current_version(current)
        .build()
        .map_err(|err| self_update_error("configure self-update", err))?
        .update()
        .map_err(|err| self_update_error("perform self-update", err))?;

    let new_version = status.version();

    if new_version == current {
        ui::success(format!("govctl v{current} is already up to date"));
    } else {
        ui::success(format!("govctl updated: v{current} -> v{new_version}"));
    }

    Ok(vec![])
}

fn self_update_error(action: &str, err: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0901IoError,
        format!("Failed to {action}: {err}"),
        "self-update",
    )
}

#[cfg(test)]
#[path = "self_update_tests.rs"]
mod tests;
