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
    reject_unmigrated_conformance(config).map_err(|error| vec![error])?;
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

    match crate::parse::load_conformance_cases_with_warnings(config) {
        Ok(result) => {
            index.conformance_cases = result.items;
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

pub(crate) fn reject_unmigrated_conformance(
    config: &Config,
) -> crate::diagnostic::DiagnosticResult<()> {
    if config.schema.version == 3 && conformance_files_present(config)? {
        return Err(Diagnostic::new(
            crate::diagnostic::DiagnosticCode::E0505MigrationRequired,
            "Schema version 3 cannot load Conformance Cases. Run `govctl migrate`.",
            config
                .display_path(&config.conformance_dir())
                .display()
                .to_string(),
        ));
    }
    Ok(())
}

fn conformance_files_present(config: &Config) -> Result<bool, Diagnostic> {
    let dir = config.conformance_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(Diagnostic::io_error(
                "read Conformance Case directory",
                error,
                config.display_path(&dir).display().to_string(),
            ));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| {
            Diagnostic::io_error(
                "read Conformance Case directory entry",
                error,
                config.display_path(&dir).display().to_string(),
            )
        })?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            return Ok(true);
        }
    }
    Ok(false)
}
