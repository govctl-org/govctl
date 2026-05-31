//! Artifact creation helpers for the `new` command.

use crate::NewTarget;
use crate::config::{Config, IdStrategy};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{
    AdrContent, AdrMeta, AdrSpec, AdrStatus, ChangelogEntry, ClauseKind, ClauseSpec, ClauseStatus,
    ClauseWire, RfcPhase, RfcSpec, RfcStatus, RfcWire, SectionSpec, WorkItemContent, WorkItemMeta,
    WorkItemSpec, WorkItemStatus, WorkItemVerification,
};
use crate::schema::{ArtifactSchema, with_schema_header};
use crate::ui;
use crate::write::{WriteOp, create_dir_all, today, write_file};
use slug::slugify;

/// Create a new artifact.
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

fn create_rfc(
    config: &Config,
    title: &str,
    manual_id: Option<&str>,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfcs_dir = config.rfc_dir();

    let rfc_id = match manual_id {
        Some(id) => {
            if !id.starts_with("RFC-") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0110RfcInvalidId,
                    format!("RFC ID must start with 'RFC-' (got: {id})"),
                    id,
                )
                .into());
            }
            if !op.is_preview() && rfcs_dir.join(id).exists() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0109RfcAlreadyExists,
                    format!("RFC already exists: {id}"),
                    id,
                )
                .into());
            }
            id.to_string()
        }
        None => {
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

    if !op.is_preview() && rfc_dir.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0109RfcAlreadyExists,
            format!("RFC already exists: {}", rfc_dir.display()),
            rfc_dir.display().to_string(),
        )
        .into());
    }

    let display_clauses_dir = config.display_path(&clauses_dir);
    create_dir_all(&clauses_dir, op, Some(&display_clauses_dir))?;

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
        refs: vec![],
        tags: vec![],
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
        signature: None, // Will be set on first bump per [[ADR-0016]]
    };

    let rfc_toml = rfc_dir.join("rfc.toml");
    let wire: RfcWire = rfc.into();
    let body = toml::to_string_pretty(&wire)?;
    let content = with_schema_header(ArtifactSchema::Rfc, &body);
    let display_rfc_toml = config.display_path(&rfc_toml);
    write_file(&rfc_toml, &content, op, Some(&display_rfc_toml))?;

    if !op.is_preview() {
        ui::created("RFC", &config.display_path(&rfc_toml));
        ui::sub_info(format!(
            "Clauses dir: {}",
            config.display_path(&clauses_dir).display()
        ));
    }

    Ok(vec![])
}

fn create_clause(
    config: &Config,
    clause_id: &str,
    title: &str,
    section: &str,
    kind: ClauseKind,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let parts: Vec<&str> = clause_id.split(':').collect();
    if parts.len() != 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            "Invalid clause ID format. Expected RFC-NNNN:C-NAME",
            clause_id,
        )
        .into());
    }

    let rfc_id = parts[0];
    let clause_name = parts[1];

    let rfc_path = config.rfc_dir().join(rfc_id).join("rfc.toml");
    if !rfc_path.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {rfc_id}"),
            rfc_id,
        )
        .into());
    }

    let mut rfc = crate::write::read_rfc(config, &rfc_path)?;

    let clause = ClauseSpec {
        clause_id: clause_name.to_string(),
        title: title.to_string(),
        kind,
        status: ClauseStatus::Active,
        text: "TODO: Add clause text here.".to_string(),
        anchors: vec![],
        superseded_by: None,
        since: None, // Will be set by rfc bump
        tags: vec![],
    };

    let clause_path = config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.toml"));

    let wire: ClauseWire = clause.into();
    let body = toml::to_string_pretty(&wire)?;
    let content = with_schema_header(ArtifactSchema::Clause, &body);
    let display_clause_path = config.display_path(&clause_path);
    write_file(&clause_path, &content, op, Some(&display_clause_path))?;

    let clause_rel_path = format!("clauses/{clause_name}.toml");
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

    let wire: RfcWire = rfc.into();
    let body = toml::to_string_pretty(&wire)?;
    let rfc_content = with_schema_header(ArtifactSchema::Rfc, &body);
    let display_rfc_path = config.display_path(&rfc_path);
    write_file(&rfc_path, &rfc_content, op, Some(&display_rfc_path))?;

    if !op.is_preview() {
        ui::created("clause", &config.display_path(&clause_path));
        ui::sub_info(format!(
            "Added to section '{}', path: {}",
            section, clause_rel_path
        ));
    }

    Ok(vec![])
}

fn create_adr(config: &Config, title: &str, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    let adr_dir = config.adr_dir();
    let display_adr_dir = config.display_path(&adr_dir);
    create_dir_all(&adr_dir, op, Some(&display_adr_dir))?;

    let mut max_num = 0u32;
    if let Ok(entries) = std::fs::read_dir(&adr_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("ADR-")
                && let Some(num_str) = name_str
                    .strip_prefix("ADR-")
                    .and_then(|s| s.split('-').next())
                && let Ok(num) = num_str.parse::<u32>()
            {
                max_num = max_num.max(num);
            }
        }
    }

    let next_num = max_num + 1;
    let adr_id = format!("ADR-{next_num:04}");
    let slug = slugify(title);
    let filename = format!("{adr_id}-{slug}.toml");
    let adr_path = adr_dir.join(&filename);

    let spec = AdrSpec {
        govctl: AdrMeta {
            schema: 1,
            id: adr_id.clone(),
            title: title.to_string(),
            status: AdrStatus::Proposed,
            date: today(),
            superseded_by: None,
            refs: vec![],
            tags: vec![],
        },
        content: AdrContent {
            context: "Describe the context and problem statement.\nWhat is the issue that we're seeing that is motivating this decision?".to_string(),
            decision: "Describe the decision that was made.\nWhat is the change that we're proposing and/or doing?".to_string(),
            consequences: "Describe the resulting context after applying the decision.\nWhat becomes easier or more difficult to do because of this change?".to_string(),
            alternatives: vec![],
        },
    };

    let body = toml::to_string_pretty(&spec)?;
    let content = with_schema_header(ArtifactSchema::Adr, &body);
    let display_adr_path = config.display_path(&adr_path);
    write_file(&adr_path, &content, op, Some(&display_adr_path))?;

    if !op.is_preview() {
        ui::created("ADR", &config.display_path(&adr_path));
    }

    Ok(vec![])
}

fn create_work_item(
    config: &Config,
    title: &str,
    active: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let work_dir = config.work_dir();
    let display_work_dir = config.display_path(&work_dir);
    create_dir_all(&work_dir, op, Some(&display_work_dir))?;

    let date = today();
    let slug = slugify(title);

    let work_id = match config.work_item.id_strategy {
        IdStrategy::Sequential => {
            let id_prefix = format!("WI-{date}-");
            let max_seq = find_max_sequence(&work_dir, &id_prefix);
            format!("WI-{date}-{:03}", max_seq + 1)
        }
        IdStrategy::AuthorHash => {
            let author_hash =
                IdStrategy::get_author_hash().unwrap_or_else(IdStrategy::generate_random_suffix);
            let id_prefix = format!("WI-{date}-{author_hash}-");
            let max_seq = find_max_sequence(&work_dir, &id_prefix);
            format!("WI-{date}-{author_hash}-{:03}", max_seq + 1)
        }
        IdStrategy::Random => {
            let random_suffix = IdStrategy::generate_random_suffix();
            format!("WI-{date}-{random_suffix}")
        }
    };

    let mut filename = format!("{date}-{slug}.toml");
    let mut work_path = work_dir.join(&filename);
    let mut suffix = 1u32;

    while !op.is_preview() && work_path.exists() {
        filename = format!("{date}-{slug}-{suffix:03}.toml");
        work_path = work_dir.join(&filename);
        suffix += 1;
    }

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
            depends_on: vec![],
            tags: vec![],
        },
        content: WorkItemContent {
            description:
                "Describe the work to be done.\nWhat is the goal? What are the acceptance criteria?"
                    .to_string(),
            journal: vec![],
            acceptance_criteria: vec![],
            notes: vec![],
        },
        verification: WorkItemVerification::default(),
    };

    let body = toml::to_string_pretty(&spec)?;
    let content = with_schema_header(ArtifactSchema::WorkItem, &body);
    let display_work_path = config.display_path(&work_path);
    write_file(&work_path, &content, op, Some(&display_work_path))?;

    if !op.is_preview() {
        let display_path = config.display_path(&work_path);
        ui::created("work item", &display_path);
        ui::sub_info(format!("ID: {work_id}"));
    }

    Ok(vec![])
}

fn find_max_sequence(work_dir: &std::path::Path, id_prefix: &str) -> u32 {
    std::fs::read_dir(work_dir)
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
                .and_then(|id| id.strip_prefix(id_prefix))
                .and_then(|seq_str| seq_str.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0)
}
