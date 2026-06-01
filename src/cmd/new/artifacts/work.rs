use crate::config::{Config, IdStrategy};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{
    WorkItemContent, WorkItemMeta, WorkItemSpec, WorkItemStatus, WorkItemVerification,
};
use crate::schema::{ArtifactSchema, with_schema_header};
use crate::ui;
use crate::write::{WriteOp, create_dir_all, today, write_file};
use slug::slugify;
use std::path::Path;

pub(super) fn create(
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

    let display_work_path = config.display_path(&work_path);
    let body = toml::to_string_pretty(&spec).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0401WorkSchemaInvalid,
            format!("Failed to serialize work item TOML: {err}"),
            display_work_path.display().to_string(),
        )
    })?;
    let content = with_schema_header(ArtifactSchema::WorkItem, &body);
    write_file(&work_path, &content, op, Some(&display_work_path))?;

    if !op.is_preview() {
        let display_path = config.display_path(&work_path);
        ui::created("work item", &display_path);
        ui::sub_info(format!("ID: {work_id}"));
    }

    Ok(vec![])
}

fn find_max_sequence(work_dir: &Path, id_prefix: &str) -> u32 {
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
