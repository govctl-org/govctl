//! New command implementation - create artifacts.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::model::{
    AdrContent, AdrMeta, AdrSpec, AdrStatus, ChangelogEntry, ClauseKind, ClauseSpec, ClauseStatus,
    RfcPhase, RfcSpec, RfcStatus, SectionSpec, WorkItemContent, WorkItemMeta, WorkItemSpec,
    WorkItemStatus,
};
use crate::write::today;
use crate::NewTarget;
use slug::slugify;

/// Initialize govctl project
pub fn init_project(config: &Config, force: bool) -> anyhow::Result<Vec<Diagnostic>> {
    let config_path = std::path::Path::new("govctl.toml");

    if config_path.exists() && !force {
        anyhow::bail!("govctl.toml already exists (use -f to overwrite)");
    }

    // Write config
    std::fs::write(config_path, Config::default_toml())?;
    eprintln!("Created: govctl.toml");

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
        NewTarget::Rfc { title, id } => create_rfc(config, title, id.as_deref()),
        NewTarget::Clause {
            clause_id,
            title,
            section,
            kind,
        } => create_clause(config, clause_id, title, section, *kind),
        NewTarget::Adr { title } => create_adr(config, title),
        NewTarget::Work { title, active } => create_work_item(config, title, *active),
    }
}

/// Create a new RFC
fn create_rfc(
    config: &Config,
    title: &str,
    manual_id: Option<&str>,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfcs_dir = config.rfcs_dir();

    // Determine RFC ID: use manual if provided, otherwise auto-generate
    let rfc_id = match manual_id {
        Some(id) => {
            // Validate format
            if !id.starts_with("RFC-") {
                anyhow::bail!("RFC ID must start with 'RFC-' (got: {id})");
            }
            // Check for collision
            if rfcs_dir.join(id).exists() {
                anyhow::bail!("RFC already exists: {id}");
            }
            id.to_string()
        }
        None => {
            // Auto-generate: find max RFC number and increment
            let max_num = std::fs::read_dir(&rfcs_dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    name_str
                        .strip_prefix("RFC-")
                        .and_then(|s| s.parse::<u32>().ok())
                })
                .max()
                .unwrap_or(0);

            format!("RFC-{:04}", max_num + 1)
        }
    };

    let rfc_dir = rfcs_dir.join(&rfc_id);
    let clauses_dir = rfc_dir.join("clauses");

    // Final collision check (handles race conditions)
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
    eprintln!(
        "  Added to section '{}', path: {}",
        section, clause_rel_path
    );

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
    let filename = format!("{adr_id}-{slug}.toml");
    let adr_path = adr_dir.join(&filename);

    // Create ADR spec
    let spec = AdrSpec {
        govctl: AdrMeta {
            schema: 1,
            id: adr_id.clone(),
            title: title.to_string(),
            status: AdrStatus::Proposed,
            date: today(),
            superseded_by: None,
            refs: vec![],
        },
        content: AdrContent {
            context: "Describe the context and problem statement.\nWhat is the issue that we're seeing that is motivating this decision?".to_string(),
            decision: "Describe the decision that was made.\nWhat is the change that we're proposing and/or doing?".to_string(),
            consequences: "Describe the resulting context after applying the decision.\nWhat becomes easier or more difficult to do because of this change?".to_string(),
            alternatives: vec![],
        },
    };

    let content = toml::to_string_pretty(&spec)?;
    std::fs::write(&adr_path, content)?;
    eprintln!("Created ADR: {}", adr_path.display());

    Ok(vec![])
}

/// Create a new work item
fn create_work_item(config: &Config, title: &str, active: bool) -> anyhow::Result<Vec<Diagnostic>> {
    let work_dir = &config.paths.work_dir;
    std::fs::create_dir_all(work_dir)?;

    let date = today();
    let slug = slugify(title);

    // Find next work item ID by scanning existing IDs for today's date
    let id_prefix = format!("WI-{date}-");

    let max_seq = std::fs::read_dir(work_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension()? == "toml").then_some(path)
        })
        .filter_map(|path| std::fs::read_to_string(&path).ok())
        .filter_map(|content| {
            content
                .lines()
                .find(|line| line.starts_with("id = \""))
                .and_then(|line| line.strip_prefix("id = \""))
                .and_then(|s| s.strip_suffix('"'))
                .and_then(|id| id.strip_prefix(&id_prefix))
                .and_then(|seq_str| seq_str.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);

    let next_seq = max_seq + 1;
    let work_id = format!("WI-{date}-{next_seq:03}");

    // Find unique filename (append sequence if slug collision)
    let mut filename = format!("{date}-{slug}.toml");
    let mut work_path = work_dir.join(&filename);

    if work_path.exists() {
        filename = format!("{date}-{slug}-{next_seq}.toml");
        work_path = work_dir.join(&filename);
    }

    // Create work item spec
    let (status, start_date) = if active {
        (WorkItemStatus::Active, Some(date.clone()))
    } else {
        (WorkItemStatus::Queue, None)
    };

    let spec = WorkItemSpec {
        govctl: WorkItemMeta {
            schema: 1,
            id: work_id.clone(),
            title: title.to_string(),
            status,
            start_date,
            done_date: None,
            refs: vec![],
        },
        content: WorkItemContent {
            description:
                "Describe the work to be done.\nWhat is the goal? What are the acceptance criteria?"
                    .to_string(),
            acceptance_criteria: vec![],
            decisions: vec![],
            notes: String::new(),
        },
    };

    let content = toml::to_string_pretty(&spec)?;
    std::fs::write(&work_path, content)?;
    eprintln!("Created work item: {}", work_path.display());

    Ok(vec![])
}
