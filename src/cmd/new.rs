//! New command implementation - create artifacts.

use crate::NewTarget;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::model::{
    AdrContent, AdrMeta, AdrSpec, AdrStatus, ChangelogEntry, ClauseKind, ClauseSpec, ClauseStatus,
    RfcPhase, RfcSpec, RfcStatus, SectionSpec, WorkItemContent, WorkItemMeta, WorkItemSpec,
    WorkItemStatus,
};
use crate::ui;
use crate::write::{WriteOp, create_dir_all, today, write_file};
use slug::slugify;
use std::path::PathBuf;

/// Embedded Claude command templates for governed workflows
const GOV_COMMAND_TEMPLATE: &str = include_str!("../../assets/gov.md");
const QUICK_COMMAND_TEMPLATE: &str = include_str!("../../assets/quick.md");
const STATUS_COMMAND_TEMPLATE: &str = include_str!("../../assets/status.md");

/// Placeholder for govctl command in templates
const GOVCTL_PLACEHOLDER: &str = "{{GOVCTL}}";
/// Default replacement for govctl command
const GOVCTL_DEFAULT: &str = "govctl";

/// Initialize govctl project
pub fn init_project(config: &Config, force: bool, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    let config_path = config.paths.gov_root.join("config.toml");

    if config_path.exists() && !force && !op.is_preview() {
        anyhow::bail!(
            "{} already exists (use -f to overwrite)",
            config_path.display()
        );
    }

    // Create directories first (config lives inside gov_root)
    let dirs = [
        &config.paths.gov_root,
        &config.rfc_dir(),
        &config.schema_dir(),
        &config.rfc_output(),
        &config.adr_dir(),
        &config.work_dir(),
        &config.templates_dir(),
    ];

    for dir in dirs {
        create_dir_all(dir, op)?;
        if !op.is_preview() {
            ui::created_path(dir);
        }
    }

    // Write config after gov_root exists
    write_file(&config_path, Config::default_toml(), op)?;
    if !op.is_preview() {
        ui::created_path(&config_path);
    }

    // Create .claude/commands directory and write command templates
    let claude_commands_dir = PathBuf::from(".claude/commands");
    create_dir_all(&claude_commands_dir, op)?;
    if !op.is_preview() {
        ui::created_path(&claude_commands_dir);
    }

    // Write command templates with {{GOVCTL}} → govctl substitution
    let templates = [
        ("gov.md", GOV_COMMAND_TEMPLATE),
        ("quick.md", QUICK_COMMAND_TEMPLATE),
        ("status.md", STATUS_COMMAND_TEMPLATE),
    ];

    for (filename, template) in templates {
        let content = template.replace(GOVCTL_PLACEHOLDER, GOVCTL_DEFAULT);
        let path = claude_commands_dir.join(filename);
        write_file(&path, &content, op)?;
        if !op.is_preview() {
            ui::created_path(&path);
        }
    }

    if !op.is_preview() {
        ui::success("Project initialized");
    }
    Ok(vec![])
}

/// Create a new artifact
pub fn create(config: &Config, target: &NewTarget, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    match target {
        NewTarget::Rfc { title, id } => create_rfc(config, title, id.as_deref(), op),
        NewTarget::Clause {
            clause_id,
            title,
            section,
            kind,
        } => create_clause(config, clause_id, title, section, *kind, op),
        NewTarget::Adr { title } => create_adr(config, title, op),
        NewTarget::Work { title, active } => create_work_item(config, title, *active, op),
    }
}

/// Create a new RFC
fn create_rfc(
    config: &Config,
    title: &str,
    manual_id: Option<&str>,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfcs_dir = config.rfc_dir();

    // Determine RFC ID: use manual if provided, otherwise auto-generate
    let rfc_id = match manual_id {
        Some(id) => {
            // Validate format
            if !id.starts_with("RFC-") {
                anyhow::bail!("RFC ID must start with 'RFC-' (got: {id})");
            }
            // Check for collision (skip in preview mode)
            if !op.is_preview() && rfcs_dir.join(id).exists() {
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

    // Final collision check (skip in preview mode)
    if !op.is_preview() && rfc_dir.exists() {
        anyhow::bail!("RFC already exists: {}", rfc_dir.display());
    }

    // Create directories
    create_dir_all(&clauses_dir, op)?;

    // Create rfc.json
    let rfc = RfcSpec {
        rfc_id: rfc_id.to_string(),
        title: title.to_string(),
        version: "0.1.0".to_string(),
        status: RfcStatus::Draft,
        phase: RfcPhase::Spec,
        owners: vec![config.project.default_owner.clone()],
        created: today(),
        updated: None,
        supersedes: None,
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
            notes: Some("Initial draft".to_string()),
            added: vec![],
            changed: vec![],
            deprecated: vec![],
            removed: vec![],
            fixed: vec![],
            security: vec![],
        }],
    };

    let rfc_json = rfc_dir.join("rfc.json");
    let content = serde_json::to_string_pretty(&rfc)?;
    write_file(&rfc_json, &content, op)?;

    if !op.is_preview() {
        ui::created("RFC", &rfc_json);
        ui::sub_info(format!("Clauses dir: {}", clauses_dir.display()));
    }

    Ok(vec![])
}

/// Create a new clause
fn create_clause(
    config: &Config,
    clause_id: &str,
    title: &str,
    section: &str,
    kind: ClauseKind,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    // Parse clause_id (RFC-0001:C-NAME)
    let parts: Vec<&str> = clause_id.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid clause ID format. Expected RFC-NNNN:C-NAME");
    }

    let rfc_id = parts[0];
    let clause_name = parts[1];

    let rfc_json = config.rfc_dir().join(rfc_id).join("rfc.json");
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
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.json"));

    let content = serde_json::to_string_pretty(&clause)?;
    write_file(&clause_path, &content, op)?;

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
    write_file(&rfc_json, &rfc_content, op)?;

    if !op.is_preview() {
        ui::created("clause", &clause_path);
        ui::sub_info(format!(
            "Added to section '{}', path: {}",
            section, clause_rel_path
        ));
    }

    Ok(vec![])
}

/// Create a new ADR
fn create_adr(config: &Config, title: &str, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    // Find next ADR number
    let adr_dir = config.adr_dir();
    create_dir_all(&adr_dir, op)?;

    let mut max_num = 0u32;
    if let Ok(entries) = std::fs::read_dir(&adr_dir) {
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
    write_file(&adr_path, &content, op)?;

    if !op.is_preview() {
        ui::created("ADR", &adr_path);
    }

    Ok(vec![])
}

/// Create a new work item
fn create_work_item(
    config: &Config,
    title: &str,
    active: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let work_dir = config.work_dir();
    create_dir_all(&work_dir, op)?;

    let date = today();
    let slug = slugify(title);

    // Find next work item ID by scanning existing IDs for today's date
    let id_prefix = format!("WI-{date}-");

    let max_seq = std::fs::read_dir(&work_dir)
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

    // Find unique filename (loop until no collision)
    let mut filename = format!("{date}-{slug}.toml");
    let mut work_path = work_dir.join(&filename);
    let mut suffix = next_seq;

    while !op.is_preview() && work_path.exists() {
        filename = format!("{date}-{slug}-{suffix:03}.toml");
        work_path = work_dir.join(&filename);
        suffix += 1;
    }

    // Create work item spec
    let (status, started) = if active {
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
            created: Some(date.clone()),
            started,
            completed: None,
            refs: vec![],
        },
        content: WorkItemContent {
            description:
                "Describe the work to be done.\nWhat is the goal? What are the acceptance criteria?"
                    .to_string(),
            acceptance_criteria: vec![],
            notes: vec![],
        },
    };

    let content = toml::to_string_pretty(&spec)?;
    write_file(&work_path, &content, op)?;

    if !op.is_preview() {
        ui::created("work item", &work_path);
        ui::sub_info(format!("ID: {work_id}"));
    }

    Ok(vec![])
}
