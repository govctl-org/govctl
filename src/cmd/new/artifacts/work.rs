use super::write_new_artifact_toml;
use crate::config::{Config, IdStrategy};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{
    WorkItemContent, WorkItemMeta, WorkItemSpec, WorkItemStatus, WorkItemVerification,
};
use crate::schema::ArtifactSchema;
use crate::ui;
use crate::write::{WriteOp, create_dir_all, today};
use slug::slugify;
use std::path::Path;

pub(super) fn create(
    config: &Config,
    title: &str,
    active: bool,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let work_dir = config.work_dir();
    let display_work_dir = config.display_path(&work_dir);
    create_dir_all(&work_dir, op, Some(&display_work_dir))?;

    let date = today();
    let slug = slugify(title);

    // [[RFC-0010:C-ID-RESERVATION]]: reserve the generated ID in the shared
    // registry and allocate from the union of the local tree, live
    // reservations, and shared history. Degrades to local-only with a
    // warning when the registry is unavailable.
    let mut reservation = crate::registry::ReservationSession::begin(config, op.is_preview());

    let work_id = match config.work_item.id_strategy {
        IdStrategy::Sequential => {
            let id_prefix = format!("WI-{date}-");
            let max_seq =
                reservation.max_witnessed(&id_prefix, find_max_sequence(config, &id_prefix)?);
            reject_exhausted_sequence(max_seq, &id_prefix, &work_dir)?;
            format!("WI-{date}-{:03}", max_seq + 1)
        }
        IdStrategy::AuthorHash => {
            let author_hash =
                IdStrategy::get_author_hash().unwrap_or_else(IdStrategy::generate_random_suffix);
            let id_prefix = format!("WI-{date}-{author_hash}-");
            let max_seq =
                reservation.max_witnessed(&id_prefix, find_max_sequence(config, &id_prefix)?);
            reject_exhausted_sequence(max_seq, &id_prefix, &work_dir)?;
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

    while work_path.exists() {
        filename = format!("{date}-{slug}-{suffix:03}.toml");
        work_path = work_dir.join(&filename);
        suffix += 1;
    }

    let (status, started) = if active {
        (WorkItemStatus::Active, Some(date.clone()))
    } else {
        (WorkItemStatus::Queue, None)
    };

    let mut meta = WorkItemMeta::new(work_id.clone(), title, status);
    meta.created = Some(date.clone());
    meta.started = started;

    let spec = WorkItemSpec {
        govctl: meta,
        content: WorkItemContent {
            description:
                "Describe the work to be done.\nWhat is the goal? What are the acceptance criteria?"
                    .to_string(),
            ..WorkItemContent::default()
        },
        verification: WorkItemVerification::default(),
    };

    write_new_artifact_toml(
        config,
        &work_path,
        &spec,
        ArtifactSchema::WorkItem,
        DiagnosticCode::E0401WorkSchemaInvalid,
        "work item",
        op,
    )?;
    reservation.record(&work_id, &work_path);

    let mut warnings = reservation.into_warnings();

    // [[RFC-0010:C-PRESENCE]]: a work item created directly in active status
    // registers a presence record naming this workspace. Advisory only.
    if status == WorkItemStatus::Active {
        let mut presence = crate::registry::PresenceSession::begin(config, op.is_preview());
        presence.register(&work_id, title);
        warnings.extend(presence.into_warnings());
    }

    if !op.is_preview() {
        let display_path = config.display_path(&work_path);
        ui::created("work item", &display_path);
        ui::sub_info(format!("ID: {work_id}"));
    }

    Ok(warnings)
}

fn reject_exhausted_sequence(
    max_seq: u32,
    id_prefix: &str,
    work_dir: &Path,
) -> DiagnosticResult<()> {
    if max_seq >= 999 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0401WorkSchemaInvalid,
            format!("Work Item ID namespace exhausted at {id_prefix}999"),
            work_dir.display().to_string(),
        ));
    }
    Ok(())
}

fn find_max_sequence(config: &Config, id_prefix: &str) -> DiagnosticResult<u32> {
    Ok(crate::parse::load_work_items(config)?
        .into_iter()
        .filter_map(|entry| {
            entry
                .spec
                .govctl
                .id
                .strip_prefix(id_prefix)
                .and_then(|seq| seq.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0))
}
