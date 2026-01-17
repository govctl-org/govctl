//! YAML frontmatter parsing for ADR and Work Item files.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrEntry, AdrMeta, PhaseOsWrapper, WorkItemEntry, WorkItemMeta};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use std::path::Path;

/// Load all ADRs from the adr directory
pub fn load_adrs(config: &Config) -> Result<Vec<AdrEntry>, Diagnostic> {
    let adr_dir = &config.paths.adr_dir;
    if !adr_dir.exists() {
        return Ok(vec![]);
    }

    let mut adrs = Vec::new();
    let entries = std::fs::read_dir(adr_dir).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            adr_dir.display().to_string(),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            match load_adr(&path) {
                Ok(adr) => adrs.push(adr),
                Err(_) => continue, // Skip invalid files
            }
        }
    }

    Ok(adrs)
}

/// Load a single ADR
pub fn load_adr(path: &Path) -> Result<AdrEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);

    let frontmatter = parsed.data.ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0301AdrSchemaInvalid,
            "Missing frontmatter",
            path.display().to_string(),
        )
    })?;

    let wrapper: PhaseOsWrapper<AdrMeta> =
        frontmatter.deserialize().map_err(|e| {
            Diagnostic::new(
                DiagnosticCode::E0301AdrSchemaInvalid,
                format!("Invalid frontmatter: {e}"),
                path.display().to_string(),
            )
        })?;

    Ok(AdrEntry {
        meta: wrapper.phaseos,
        path: path.to_path_buf(),
        content: parsed.content,
    })
}

/// Load all work items from the work directory
pub fn load_work_items(config: &Config) -> Result<Vec<WorkItemEntry>, Diagnostic> {
    let work_dir = &config.paths.work_dir;
    if !work_dir.exists() {
        return Ok(vec![]);
    }

    let mut items = Vec::new();
    let entries = std::fs::read_dir(work_dir).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            work_dir.display().to_string(),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            match load_work_item(&path) {
                Ok(item) => items.push(item),
                Err(_) => continue, // Skip invalid files
            }
        }
    }

    Ok(items)
}

/// Load a single work item
pub fn load_work_item(path: &Path) -> Result<WorkItemEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);

    let frontmatter = parsed.data.ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0401WorkSchemaInvalid,
            "Missing frontmatter",
            path.display().to_string(),
        )
    })?;

    let wrapper: PhaseOsWrapper<WorkItemMeta> =
        frontmatter.deserialize().map_err(|e| {
            Diagnostic::new(
                DiagnosticCode::E0401WorkSchemaInvalid,
                format!("Invalid frontmatter: {e}"),
                path.display().to_string(),
            )
        })?;

    Ok(WorkItemEntry {
        meta: wrapper.phaseos,
        path: path.to_path_buf(),
        content: parsed.content,
    })
}

/// Update frontmatter in a markdown file
pub fn update_frontmatter<T: serde::Serialize>(
    path: &Path,
    meta: &PhaseOsWrapper<T>,
) -> Result<(), Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);

    let yaml = serde_yaml::to_string(meta).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0903YamlParseError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let new_content = format!("---\n{}---\n{}", yaml, parsed.content);

    std::fs::write(path, new_content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    Ok(())
}
