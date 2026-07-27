use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseKind, ConformanceEntry, ProjectIndex, RfcStatus};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Validate the complete current Conformance Case graph.
pub fn validate_cases(config: &Config, cases: &[ConformanceEntry]) -> Vec<Diagnostic> {
    let mut prospective = config.clone();
    prospective.schema.version = crate::cmd::migrate::CURRENT_SCHEMA_VERSION;
    let mut index = match crate::load::load_project(&prospective) {
        Ok(index) => index,
        Err(errors) => return errors,
    };
    index.conformance_cases = cases.to_vec();
    crate::validate::validate_project(&index, &prospective)
        .diagnostics
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.code == DiagnosticCode::E1305ConformanceGraphInvalid
                || diagnostic.message.contains("Conformance Case")
        })
        .collect()
}

pub(super) fn validate_cases_with_index(
    config: &Config,
    cases: &[ConformanceEntry],
    index: &ProjectIndex,
) -> Vec<Diagnostic> {
    let guards = match crate::parse::load_guards(config) {
        Ok(guards) => guards,
        Err(error) => return vec![error],
    };
    let guard_ids = guards
        .iter()
        .map(|guard| guard.meta().id.as_str())
        .collect::<HashSet<_>>();
    let allowed_tags = config
        .tags
        .allowed
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut diagnostics = Vec::new();
    let mut ids = HashMap::<&str, &Path>::new();
    let mut locators = HashMap::<(PathBuf, &str), &str>::new();

    for entry in cases {
        let id = entry.meta().id.as_str();
        let display = config.display_path(&entry.path).display().to_string();
        let expected_stem = entry.path.file_stem().and_then(|stem| stem.to_str());
        if expected_stem != Some(id) {
            diagnostics.push(invalid(
                format!("Conformance Case ID '{id}' must equal its filename stem"),
                &display,
            ));
        }
        if let Some(previous) = ids.insert(id, &entry.path) {
            diagnostics.push(invalid(
                format!(
                    "Duplicate Conformance Case ID '{id}' in '{}' and '{}'",
                    config.display_path(previous).display(),
                    display
                ),
                &display,
            ));
        }

        if entry.meta().title.trim().is_empty() {
            diagnostics.push(invalid("Conformance Case title cannot be empty", &display));
        }
        validate_unique_values("tag", &entry.meta().tags, &display, &mut diagnostics);
        for tag in &entry.meta().tags {
            if !allowed_tags.contains(tag.as_str()) {
                diagnostics.push(invalid(
                    format!("Conformance Case '{id}' uses unregistered tag '{tag}'"),
                    &display,
                ));
            }
        }
        if entry.spec.case.selector.trim().is_empty() {
            diagnostics.push(invalid(
                "Conformance Case selector cannot be empty",
                &display,
            ));
        }
        match canonical_scenario_path(config, &entry.spec.case.path) {
            Ok(path) => {
                let key = (path, entry.spec.case.selector.as_str());
                if let Some(other) = locators.insert(key, id) {
                    diagnostics.push(invalid(
                        format!(
                            "Conformance Cases '{other}' and '{id}' use the same path and selector"
                        ),
                        &display,
                    ));
                }
            }
            Err(message) => diagnostics.push(invalid(message, &display)),
        }

        if entry.spec.case.requirements.is_empty() {
            diagnostics.push(invalid(
                "Conformance Case must contain at least one requirement",
                &display,
            ));
        }
        let mut requirement_refs = HashSet::new();
        for requirement in &entry.spec.case.requirements {
            if !requirement_refs.insert(requirement.clause_ref.as_str()) {
                diagnostics.push(invalid(
                    format!(
                        "Conformance Case '{id}' repeats requirement '{}'",
                        requirement.clause_ref
                    ),
                    &display,
                ));
                continue;
            }
            validate_requirement(index, requirement, &display, &mut diagnostics);
        }

        validate_unique_values("Guard", &entry.spec.case.guards, &display, &mut diagnostics);
        for guard in &entry.spec.case.guards {
            if !guard_ids.contains(guard.as_str()) {
                diagnostics.push(invalid(
                    format!("Conformance Case '{id}' references unknown Guard '{guard}'"),
                    &display,
                ));
            }
        }
    }

    for guard in guards {
        for reference in &guard.meta().refs {
            if cases.iter().any(|case| case.meta().id == *reference) {
                diagnostics.push(invalid(
                    format!(
                        "Verification Guard '{}' cannot reference Conformance Case '{}'",
                        guard.meta().id,
                        reference
                    ),
                    &config.display_path(&guard.path).display().to_string(),
                ));
            }
        }
    }

    diagnostics
}

fn validate_requirement(
    index: &ProjectIndex,
    requirement: &crate::model::RequirementBinding,
    display: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((rfc_id, clause_id)) = requirement.clause_ref.split_once(':') else {
        diagnostics.push(invalid(
            format!(
                "Requirement '{}' is not a fully qualified Clause ID",
                requirement.clause_ref
            ),
            display,
        ));
        return;
    };
    let Some(rfc) = index.rfcs.iter().find(|entry| entry.rfc.rfc_id == rfc_id) else {
        diagnostics.push(invalid(
            format!("Requirement RFC '{rfc_id}' does not exist"),
            display,
        ));
        return;
    };
    let Some(clause) = rfc
        .clauses
        .iter()
        .find(|entry| entry.spec.clause_id == clause_id)
    else {
        diagnostics.push(invalid(
            format!(
                "Requirement Clause '{}' does not exist",
                requirement.clause_ref
            ),
            display,
        ));
        return;
    };
    if clause.spec.kind == ClauseKind::Informative {
        diagnostics.push(invalid(
            format!(
                "Requirement '{}' identifies an informative Clause",
                requirement.clause_ref
            ),
            display,
        ));
    }

    let Ok(version) = semver::Version::parse(&requirement.version) else {
        diagnostics.push(invalid(
            format!(
                "Requirement version '{}' is not semantic versioning",
                requirement.version
            ),
            display,
        ));
        return;
    };
    let version_exists = rfc
        .rfc
        .changelog
        .iter()
        .any(|entry| entry.version == requirement.version);
    if !version_exists {
        diagnostics.push(invalid(
            format!(
                "Requirement version '{}' is absent from {} changelog",
                requirement.version, rfc_id
            ),
            display,
        ));
    }

    match &clause.spec.since {
        Some(since) => {
            if semver::Version::parse(since).is_ok_and(|since| version < since) {
                diagnostics.push(invalid(
                    format!(
                        "Requirement version '{}' predates Clause {} since '{}'",
                        requirement.version, requirement.clause_ref, since
                    ),
                    display,
                ));
            }
        }
        None if rfc.rfc.status != RfcStatus::Draft => diagnostics.push(invalid(
            format!(
                "Requirement '{}' targets a pending Clause outside a draft RFC",
                requirement.clause_ref
            ),
            display,
        )),
        None => {}
    }
    if rfc.rfc.status == RfcStatus::Draft && requirement.version != rfc.rfc.version {
        diagnostics.push(invalid(
            format!(
                "Draft requirement '{}' must bind current RFC version '{}'",
                requirement.clause_ref, rfc.rfc.version
            ),
            display,
        ));
    }
}

fn canonical_scenario_path(config: &Config, value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err("Conformance Case path cannot be empty".to_string());
    }
    let relative = Path::new(value);
    if relative.is_absolute() {
        return Err(format!(
            "Conformance Case path '{value}' must be repository-relative"
        ));
    }
    let root = config
        .project_root()
        .canonicalize()
        .map_err(|err| format!("Cannot resolve project root: {err}"))?;
    let target = root
        .join(relative)
        .canonicalize()
        .map_err(|err| format!("Cannot resolve Conformance Case path '{value}': {err}"))?;
    if !target.starts_with(&root) {
        return Err(format!(
            "Conformance Case path '{value}' escapes the project root"
        ));
    }
    let gov_root = config
        .gov_root
        .canonicalize()
        .map_err(|err| format!("Cannot resolve gov root: {err}"))?;
    if target.starts_with(gov_root) {
        return Err(format!(
            "Conformance Case path '{value}' resolves inside the gov root"
        ));
    }
    if !target.is_file() {
        return Err(format!(
            "Conformance Case path '{value}' is not a regular file"
        ));
    }
    Ok(target)
}

fn validate_unique_values(
    label: &str,
    values: &[String],
    display: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value.as_str()) {
            diagnostics.push(invalid(
                format!("Conformance Case repeats {label} '{value}'"),
                display,
            ));
        }
    }
}

fn invalid(message: impl Into<String>, source: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1305ConformanceGraphInvalid,
        message,
        source,
    )
}
