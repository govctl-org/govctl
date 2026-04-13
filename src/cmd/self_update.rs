//! Self-update command: download and replace the govctl binary from GitHub Releases.
//!
//! Implements [[RFC-0002:C-SELF-UPDATE]].

use std::io::IsTerminal;

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::ui;

const REPO_OWNER: &str = "govctl-org";
const REPO_NAME: &str = "govctl";

/// Result of comparing the current version against the latest available.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum VersionCheck {
    /// Current version is up to date (latest <= current).
    UpToDate,
    /// A newer version is available.
    UpdateAvailable { current: String, latest: String },
}

/// Compare two semver version strings. Returns whether an update is available.
pub(crate) fn compare_versions(current: &str, latest_raw: &str) -> anyhow::Result<VersionCheck> {
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
pub fn self_update(check_only: bool) -> anyhow::Result<Vec<Diagnostic>> {
    let current = env!("CARGO_PKG_VERSION");

    if check_only {
        check_version(current)
    } else {
        perform_update(current)
    }
}

fn check_version(current: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;

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

fn perform_update(current: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let show_progress = std::io::stdout().is_terminal();

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("govctl")
        .show_download_progress(show_progress)
        .current_version(current)
        .build()?
        .update()?;

    let new_version = status.version();

    if new_version == current {
        ui::success(format!("govctl v{current} is already up to date"));
    } else {
        ui::success(format!("govctl updated: v{current} -> v{new_version}"));
    }

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_version_is_up_to_date() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(compare_versions("0.8.3", "0.8.3")?, VersionCheck::UpToDate);
        Ok(())
    }

    #[test]
    fn test_newer_available() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            compare_versions("0.8.2", "0.8.3")?,
            VersionCheck::UpdateAvailable {
                current: "0.8.2".into(),
                latest: "0.8.3".into(),
            }
        );
        Ok(())
    }

    #[test]
    fn test_current_newer_than_latest_is_up_to_date() -> Result<(), Box<dyn std::error::Error>> {
        // Dev build ahead of latest release
        assert_eq!(compare_versions("0.9.0", "0.8.3")?, VersionCheck::UpToDate);
        Ok(())
    }

    #[test]
    fn test_strips_v_prefix() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(compare_versions("0.8.3", "v0.8.3")?, VersionCheck::UpToDate);
        assert_eq!(
            compare_versions("0.8.2", "v0.9.0")?,
            VersionCheck::UpdateAvailable {
                current: "0.8.2".into(),
                latest: "0.9.0".into(),
            }
        );
        Ok(())
    }

    #[test]
    fn test_major_version_update() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            compare_versions("0.8.3", "1.0.0")?,
            VersionCheck::UpdateAvailable {
                current: "0.8.3".into(),
                latest: "1.0.0".into(),
            }
        );
        Ok(())
    }

    #[test]
    fn test_prerelease_not_newer_than_release() -> Result<(), Box<dyn std::error::Error>> {
        // 1.0.0-alpha < 1.0.0 per semver, so if current is 1.0.0 and latest is 1.0.0-alpha
        assert_eq!(
            compare_versions("1.0.0", "1.0.0-alpha")?,
            VersionCheck::UpToDate
        );
        Ok(())
    }

    #[test]
    fn test_invalid_current_version_errors() {
        assert!(compare_versions("not-a-version", "0.8.3").is_err());
    }

    #[test]
    fn test_invalid_latest_version_errors() {
        assert!(compare_versions("0.8.3", "not-a-version").is_err());
    }
}
