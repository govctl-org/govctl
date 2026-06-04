use super::ProjectLoadResult;
use super::rfc::load_rfcs;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::model::ProjectIndex;

/// Load full project index (RFCs, ADRs, Work Items)
pub fn load_project(config: &Config) -> Result<ProjectIndex, Vec<Diagnostic>> {
    load_project_with_warnings(config).map(|r| r.index)
}

/// Load full project index, returning both the index and any parse warnings
pub fn load_project_with_warnings(config: &Config) -> Result<ProjectLoadResult, Vec<Diagnostic>> {
    let mut index = ProjectIndex::default();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    match load_rfcs(config) {
        Ok(rfcs) => index.rfcs = rfcs,
        Err(e) => errors.push(e.into()),
    }

    match crate::parse::load_adrs_with_warnings(config) {
        Ok(result) => {
            index.adrs = result.items;
            warnings.extend(result.warnings);
        }
        Err(e) => errors.push(e),
    }

    match crate::parse::load_work_items_with_warnings(config) {
        Ok(result) => {
            index.work_items = result.items;
            warnings.extend(result.warnings);
        }
        Err(e) => errors.push(e),
    }

    if errors.is_empty() {
        Ok(ProjectLoadResult { index, warnings })
    } else {
        Err(errors)
    }
}
