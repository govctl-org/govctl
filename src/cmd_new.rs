//! New command implementation - create artifacts.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::model::{
    AdrMeta, AdrStatus, ChangelogEntry, ClauseKind, ClauseSpec, ClauseStatus, PhaseOsWrapper,
    RfcPhase, RfcSpec, RfcStatus, SectionSpec, WorkItemMeta, WorkItemStatus,
};
use crate::write::today;
use crate::NewTarget;
use slug::slugify;

/// Initialize phaseos project
pub fn init_project(config: &Config, force: bool) -> anyhow::Result<Vec<Diagnostic>> {
    let config_path = std::path::Path::new("phaseos.toml");

    if config_path.exists() && !force {
        anyhow::bail!("phaseos.toml already exists (use -f to overwrite)");
    }

    // Write config
    std::fs::write(config_path, Config::default_toml())?;
    eprintln!("Created: phaseos.toml");

    // Create directories
    let dirs = [
        &config.paths.spec_root,
        &config.rfcs_dir(),
        &config.schema_dir(),
        &config.paths.rfc_output,
        &config.paths.adr_dir,
        &config.paths.work_dir,
        &config.paths.templates_dir,
    ];

    for dir in dirs {
        std::fs::create_dir_all(dir)?;
        eprintln!("Created: {}", dir.display());
    }

    eprintln!("✓ Project initialized");
    Ok(vec![])
}

/// Create a new artifact
pub fn create(config: &Config, target: &NewTarget) -> anyhow::Result<Vec<Diagnostic>> {
    match target {
        NewTarget::Rfc { rfc_id, title } => create_rfc(config, rfc_id, title),
        NewTarget::Clause {
            clause_id,
            title,
            section,
            kind,
        } => create_clause(config, clause_id, title, section, *kind),
        NewTarget::Adr { title } => create_adr(config, title),
        NewTarget::Work { title } => create_work_item(config, title),
    }
}

/// Create a new RFC
fn create_rfc(config: &Config, rfc_id: &str, title: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let rfc_dir = config.rfcs_dir().join(rfc_id);
    let clauses_dir = rfc_dir.join("clauses");

    if rfc_dir.exists() {
        anyhow::bail!("RFC already exists: {}", rfc_dir.display());
    }

    // Create directories
    std::fs::create_dir_all(&clauses_dir)?;

    // Create rfc.json
    let rfc = RfcSpec {
        rfc_id: rfc_id.to_string(),
        title: title.to_string(),
        version: "0.1.0".to_string(),
        status: RfcStatus::Draft,
        phase: RfcPhase::Spec,
        owners: vec!["@your-handle".to_string()],
        created: today(),
        updated: None,
        sections: vec![
            SectionSpec {
                title: "Summary".to_string(),
                clauses: vec![],
            },
            SectionSpec {
                title: "Specification".to_string(),
                clauses: vec![],
            },
        ],
        changelog: vec![ChangelogEntry {
            version: "0.1.0".to_string(),
            date: today(),
            summary: "Initial draft".to_string(),
            changes: vec![],
        }],
    };

    let rfc_json = rfc_dir.join("rfc.json");
    let content = serde_json::to_string_pretty(&rfc)?;
    std::fs::write(&rfc_json, content)?;

    eprintln!("Created RFC: {}", rfc_json.display());
    eprintln!("  Clauses dir: {}", clauses_dir.display());

    Ok(vec![])
}

/// Create a new clause
fn create_clause(
    config: &Config,
    clause_id: &str,
    title: &str,
    section: &str,
    kind: ClauseKind,
) -> anyhow::Result<Vec<Diagnostic>> {
    // Parse clause_id (RFC-0001:C-NAME)
    let parts: Vec<&str> = clause_id.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid clause ID format. Expected RFC-NNNN:C-NAME");
    }

    let rfc_id = parts[0];
    let clause_name = parts[1];

    let rfc_json = config.rfcs_dir().join(rfc_id).join("rfc.json");
    if !rfc_json.exists() {
        anyhow::bail!("RFC not found: {rfc_id}");
    }

    // Create clause
    let clause = ClauseSpec {
        clause_id: clause_name.to_string(),
        title: title.to_string(),
        kind,
        status: ClauseStatus::Active,
        text: "TODO: Add clause text here.".to_string(),
        anchors: vec![],
        superseded_by: None,
        since: None, // Will be set on next version bump
    };

    let clause_path = config
        .rfcs_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.json"));

    let content = serde_json::to_string_pretty(&clause)?;
    std::fs::write(&clause_path, content)?;

    // Update RFC to include clause in section
    let mut rfc: RfcSpec = serde_json::from_str(&std::fs::read_to_string(&rfc_json)?)?;

    let clause_rel_path = format!("clauses/{clause_name}.json");

    // Find or create section
    if let Some(sec) = rfc.sections.iter_mut().find(|s| s.title == section) {
        if !sec.clauses.contains(&clause_rel_path) {
            sec.clauses.push(clause_rel_path.clone());
        }
    } else {
        rfc.sections.push(SectionSpec {
            title: section.to_string(),
            clauses: vec![clause_rel_path.clone()],
        });
    }

    // Write updated RFC
    let rfc_content = serde_json::to_string_pretty(&rfc)?;
    std::fs::write(&rfc_json, rfc_content)?;

    eprintln!("Created clause: {}", clause_path.display());
    eprintln!("  Added to section '{}', path: {}", section, clause_rel_path);

    Ok(vec![])
}

/// Create a new ADR
fn create_adr(config: &Config, title: &str) -> anyhow::Result<Vec<Diagnostic>> {
    // Find next ADR number
    let adr_dir = &config.paths.adr_dir;
    std::fs::create_dir_all(adr_dir)?;

    let mut max_num = 0u32;
    if let Ok(entries) = std::fs::read_dir(adr_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("ADR-") {
                if let Some(num_str) = name_str
                    .strip_prefix("ADR-")
                    .and_then(|s| s.split('-').next())
                {
                    if let Ok(num) = num_str.parse::<u32>() {
                        max_num = max_num.max(num);
                    }
                }
            }
        }
    }

    let next_num = max_num + 1;
    let adr_id = format!("ADR-{next_num:04}");
    let slug = slugify(title);
    let filename = format!("{adr_id}-{slug}.md");
    let adr_path = adr_dir.join(&filename);

    // Create ADR content
    let meta = PhaseOsWrapper {
        phaseos: AdrMeta {
            schema: 1,
            id: adr_id.clone(),
            title: title.to_string(),
            kind: "adr".to_string(),
            status: AdrStatus::Proposed,
            date: today(),
            superseded_by: None,
            refs: vec![],
        },
        ext: None,
    };

    let yaml = serde_yaml::to_string(&meta)?;
    let content = format!(
        r#"---
{yaml}---

# {adr_id}: {title}

## Context

[Describe the context and problem statement.]

## Decision

[Describe the decision that was made.]

## Consequences

[Describe the consequences of this decision.]

## References

- [Link to relevant RFCs or documents]
"#
    );

    std::fs::write(&adr_path, content)?;
    eprintln!("Created ADR: {}", adr_path.display());

    Ok(vec![])
}

/// Create a new work item
fn create_work_item(config: &Config, title: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let work_dir = &config.paths.work_dir;
    std::fs::create_dir_all(work_dir)?;

    let date = today();
    let slug = slugify(title);

    // Find unique filename
    let mut counter = 1u32;
    let mut filename = format!("{date}-{slug}.md");
    let mut work_path = work_dir.join(&filename);

    while work_path.exists() {
        counter += 1;
        filename = format!("{date}-{slug}-{counter}.md");
        work_path = work_dir.join(&filename);
    }

    let work_id = format!("WI-{date}-{counter:03}");

    // Create work item content
    let meta = PhaseOsWrapper {
        phaseos: WorkItemMeta {
            schema: 1,
            id: work_id.clone(),
            title: title.to_string(),
            kind: "work".to_string(),
            status: WorkItemStatus::Queue,
            start_date: None,
            done_date: None,
            refs: vec![],
        },
        ext: None,
    };

    let yaml = serde_yaml::to_string(&meta)?;
    let content = format!(
        r#"---
{yaml}---

# {title}

## Goal

[Describe what this work item aims to accomplish.]

## Tasks

- [ ] Task 1
- [ ] Task 2

## Notes

[Add notes as work progresses.]
"#
    );

    std::fs::write(&work_path, content)?;
    eprintln!("Created work item: {}", work_path.display());

    Ok(vec![])
}
