use super::ops::FileOp;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{ClauseSpec, ClauseWire, RfcSpec, RfcWire};
use crate::schema::{ArtifactSchema, validate_toml_value, with_schema_header};
use crate::write::{read_clause, read_rfc};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(super) fn plan_rfc_json_to_toml(
    config: &Config,
    rfc_dir: &Path,
) -> DiagnosticResult<Option<(Vec<FileOp>, String)>> {
    let rfc_json = rfc_dir.join("rfc.json");
    let rfc_toml = rfc_dir.join("rfc.toml");

    if rfc_toml.exists() {
        if rfc_json.exists() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0101RfcSchemaInvalid,
                format!(
                    "Mixed RFC storage detected in {}: both rfc.json and rfc.toml exist",
                    config.display_path(rfc_dir).display()
                ),
                config.display_path(rfc_dir).display().to_string(),
            ));
        }
        return Ok(None);
    }

    if !rfc_json.exists() {
        return Ok(None);
    }

    for entry in fs::read_dir(rfc_dir).map_err(|err| {
        Diagnostic::io_error(
            "read RFC directory for migration",
            err,
            config.display_path(rfc_dir).display().to_string(),
        )
    })? {
        let entry = entry.map_err(|err| {
            Diagnostic::io_error(
                "read RFC directory entry for migration",
                err,
                config.display_path(rfc_dir).display().to_string(),
            )
        })?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "rfc.json" || file_name == "clauses" {
            continue;
        }
        return Err(Diagnostic::new(
            DiagnosticCode::E0101RfcSchemaInvalid,
            format!(
                "Unexpected file in RFC directory during migration: {}",
                file_name
            ),
            config.display_path(&entry.path()).display().to_string(),
        ));
    }

    let mut rfc: RfcSpec = read_rfc(config, &rfc_json)?;
    let clauses_dir = rfc_dir.join("clauses");
    let mut clause_map: BTreeMap<String, ClauseSpec> = BTreeMap::new();
    let mut ops = Vec::new();

    if clauses_dir.exists() {
        for entry in fs::read_dir(&clauses_dir).map_err(|err| {
            Diagnostic::io_error(
                "read clauses directory for migration",
                err,
                config.display_path(&clauses_dir).display().to_string(),
            )
        })? {
            let entry = entry.map_err(|err| {
                Diagnostic::io_error(
                    "read clause directory entry for migration",
                    err,
                    config.display_path(&clauses_dir).display().to_string(),
                )
            })?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!(
                        "Mixed clause storage in {}: TOML clause exists before migration",
                        config.display_path(&clauses_dir).display()
                    ),
                    config.display_path(&path).display().to_string(),
                ));
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!("Unexpected file in clauses directory: {name}"),
                    config.display_path(&path).display().to_string(),
                ));
            }
            let clause = read_clause(config, &path)?;
            clause_map.insert(name, clause);
        }
    }

    for section in &mut rfc.sections {
        for clause_path in &mut section.clauses {
            if clause_path.contains("..") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0204ClausePathInvalid,
                    format!("Invalid clause path: {clause_path}"),
                    config.display_path(&rfc_json).display().to_string(),
                ));
            }
            if !clause_path.ends_with(".json") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0204ClausePathInvalid,
                    format!("Mixed clause path formats not supported: {clause_path}"),
                    config.display_path(&rfc_json).display().to_string(),
                ));
            }
            let file_name = Path::new(clause_path)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::E0204ClausePathInvalid,
                        format!("Invalid clause path: {clause_path}"),
                        config.display_path(&rfc_json).display().to_string(),
                    )
                })?;
            if !clause_map.contains_key(file_name) {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0202ClauseNotFound,
                    format!("Referenced clause missing: {clause_path}"),
                    config.display_path(&rfc_json).display().to_string(),
                ));
            }
            *clause_path = clause_path.trim_end_matches(".json").to_string() + ".toml";
        }
    }

    let rfc_id = rfc.rfc_id.clone();
    let rfc_wire: RfcWire = rfc.into();
    let rfc_toml_path = rfc_dir.join("rfc.toml");
    let display_rfc_toml = config.display_path(&rfc_toml_path).display().to_string();
    let rfc_body = toml::to_string_pretty(&rfc_wire).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0101RfcSchemaInvalid,
            format!("Failed to serialize RFC TOML: {err}"),
            &display_rfc_toml,
        )
    })?;
    let rfc_raw: toml::Value = toml::from_str(&rfc_body).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0101RfcSchemaInvalid,
            format!("Failed to parse generated RFC TOML: {err}"),
            &display_rfc_toml,
        )
    })?;
    validate_toml_value(ArtifactSchema::Rfc, config, &rfc_toml_path, &rfc_raw)?;

    ops.push(FileOp::Write {
        path: rfc_toml_path,
        content: with_schema_header(ArtifactSchema::Rfc, &rfc_body),
    });
    ops.push(FileOp::Delete { path: rfc_json });

    for (file_name, clause) in clause_map {
        let toml_name = file_name.trim_end_matches(".json").to_string() + ".toml";
        let clause_toml_path = clauses_dir.join(&toml_name);
        let display_clause_toml = config.display_path(&clause_toml_path).display().to_string();
        let clause_wire: ClauseWire = clause.into();
        let body = toml::to_string_pretty(&clause_wire).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0201ClauseSchemaInvalid,
                format!("Failed to serialize clause TOML: {err}"),
                &display_clause_toml,
            )
        })?;
        let raw: toml::Value = toml::from_str(&body).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0201ClauseSchemaInvalid,
                format!("Failed to parse generated clause TOML: {err}"),
                &display_clause_toml,
            )
        })?;
        validate_toml_value(ArtifactSchema::Clause, config, &clause_toml_path, &raw)?;

        ops.push(FileOp::Write {
            path: clause_toml_path,
            content: with_schema_header(ArtifactSchema::Clause, &body),
        });
        ops.push(FileOp::Delete {
            path: clauses_dir.join(&file_name),
        });
    }

    Ok(Some((ops, rfc_id)))
}
