use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrContent, AdrMeta, AdrSpec, AdrStatus};
use crate::schema::{ArtifactSchema, with_schema_header};
use crate::ui;
use crate::write::{WriteOp, create_dir_all, today, write_file};
use slug::slugify;

pub(super) fn create(config: &Config, title: &str, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
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
        govctl: AdrMeta::new(adr_id.clone(), title, AdrStatus::Proposed, today()),
        content: AdrContent {
            context: "Describe the context and problem statement.\nWhat is the issue that we're seeing that is motivating this decision?".to_string(),
            decision: "Describe the decision that was made.\nWhat is the change that we're proposing and/or doing?".to_string(),
            consequences: "Describe the resulting context after applying the decision.\nWhat becomes easier or more difficult to do because of this change?".to_string(),
            alternatives: vec![],
        },
    };

    let display_adr_path = config.display_path(&adr_path);
    let body = toml::to_string_pretty(&spec).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0301AdrSchemaInvalid,
            format!("Failed to serialize ADR TOML: {err}"),
            display_adr_path.display().to_string(),
        )
    })?;
    let content = with_schema_header(ArtifactSchema::Adr, &body);
    write_file(&adr_path, &content, op, Some(&display_adr_path))?;

    if !op.is_preview() {
        ui::created("ADR", &config.display_path(&adr_path));
    }

    Ok(vec![])
}
