use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ProjectIndex, ReleasesFile};
use std::collections::HashSet;

pub fn validate_releases(
    releases: &ReleasesFile,
    index: &ProjectIndex,
    config: &Config,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_versions = HashSet::new();
    let known_work_ids: HashSet<&str> = index
        .work_items
        .iter()
        .map(|work| work.meta().id.as_str())
        .collect();
    let releases_display = config
        .display_path(&config.releases_path())
        .display()
        .to_string();

    for release in &releases.releases {
        if !seen_versions.insert(release.version.as_str()) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0702ReleaseDuplicate,
                format!("Duplicate release version: {}", release.version),
                releases_display.clone(),
            ));
        }

        for work_id in &release.refs {
            if !known_work_ids.contains(work_id.as_str()) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0705ReleaseRefNotFound,
                    format!(
                        "Release '{}' references unknown work item: {}",
                        release.version, work_id
                    ),
                    releases_display.clone(),
                ));
            }
        }
    }

    diagnostics
}
