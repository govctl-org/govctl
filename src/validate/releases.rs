use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ProjectIndex, ReleasesFile, WorkItemStatus};
use std::collections::{HashMap, HashSet};

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
    let work_statuses: HashMap<&str, WorkItemStatus> = index
        .work_items
        .iter()
        .map(|work| (work.meta().id.as_str(), work.meta().status))
        .collect();
    let mut released_work = HashMap::new();
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
                continue;
            }

            if work_statuses.get(work_id.as_str()) != Some(&WorkItemStatus::Done) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0706ReleaseWorkNotDone,
                    format!(
                        "Release '{}' references Work Item '{}' with status other than done",
                        release.version, work_id
                    ),
                    releases_display.clone(),
                ));
            }

            if let Some(first_version) = released_work.insert(work_id.as_str(), &release.version)
                && first_version != &release.version
            {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0707ReleaseWorkDuplicate,
                    format!(
                        "Work Item '{}' is referenced by releases '{}' and '{}'",
                        work_id, first_version, release.version
                    ),
                    releases_display.clone(),
                ));
            }
        }
    }

    diagnostics
}
