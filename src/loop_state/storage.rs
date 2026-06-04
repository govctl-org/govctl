use super::validation::{invalid_state, validate_loop_id};
use super::{LoopRoundRecord, LoopState};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::write::WriteOp;
use std::path::{Path, PathBuf};

pub fn loop_state_path(config: &Config, loop_id: &str) -> DiagnosticResult<PathBuf> {
    validate_loop_id(loop_id)?;
    Ok(loop_state_dir(config, loop_id)?.join("state.toml"))
}

pub fn loop_round_path(
    config: &Config,
    loop_id: &str,
    round_number: u32,
) -> DiagnosticResult<PathBuf> {
    validate_loop_id(loop_id)?;
    if round_number == 0 {
        return Err(invalid_state(
            loop_id,
            "loop round path round_number must be at least 1",
        ));
    }
    Ok(loop_state_dir(config, loop_id)?
        .join("rounds")
        .join(format!("round-{round_number:03}.toml")))
}

fn loop_state_dir(config: &Config, loop_id: &str) -> DiagnosticResult<PathBuf> {
    validate_loop_id(loop_id)?;
    Ok(loop_state_root(config).join(loop_id))
}

pub fn loop_state_root(config: &Config) -> PathBuf {
    project_root(config).join(".govctl").join("loops")
}

pub fn write_loop_state_with_op(
    config: &Config,
    state: &LoopState,
    op: WriteOp,
) -> DiagnosticResult<()> {
    state.validate(Some(&state.loop_meta.id))?;
    let path = loop_state_path(config, &state.loop_meta.id)?;
    write_loop_toml(
        config,
        &path,
        state,
        op,
        "Loop state path has no parent directory",
        "Failed to serialize loop state",
    )
}

pub fn load_loop_state(config: &Config, loop_id: &str) -> DiagnosticResult<LoopState> {
    let path = loop_state_path(config, loop_id)?;
    let body = std::fs::read_to_string(&path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E1202LoopStateNotFound,
            format!("Failed to read loop state: {e}"),
            path.display().to_string(),
        )
    })?;
    let state: LoopState = toml::from_str(&body).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            format!("Invalid loop state TOML: {e}"),
            path.display().to_string(),
        )
    })?;
    state.validate(Some(loop_id))?;
    Ok(state)
}

pub fn load_loop_round_record(
    config: &Config,
    loop_id: &str,
    round_number: u32,
) -> DiagnosticResult<LoopRoundRecord> {
    let path = loop_round_path(config, loop_id, round_number)?;
    let body = std::fs::read_to_string(&path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E1202LoopStateNotFound,
            format!("Failed to read loop round record: {e}"),
            path.display().to_string(),
        )
    })?;
    let record: LoopRoundRecord = toml::from_str(&body).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            format!("Invalid loop round TOML: {e}"),
            path.display().to_string(),
        )
    })?;
    record.validate()?;
    if record.round_meta.loop_id != loop_id {
        return Err(Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            format!(
                "round.loop_id '{}' does not match loop directory '{}'",
                record.round_meta.loop_id, loop_id
            ),
            path.display().to_string(),
        ));
    }
    if record.round_meta.round_number != round_number {
        return Err(Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            format!(
                "round.round_number {} does not match round path {}",
                record.round_meta.round_number, round_number
            ),
            path.display().to_string(),
        ));
    }
    Ok(record)
}

pub fn write_loop_round_record(
    config: &Config,
    record: &LoopRoundRecord,
    op: WriteOp,
) -> DiagnosticResult<()> {
    record.validate()?;
    let path = loop_round_path(
        config,
        &record.round_meta.loop_id,
        record.round_meta.round_number,
    )?;
    write_loop_toml(
        config,
        &path,
        record,
        op,
        "Loop round path has no parent directory",
        "Failed to serialize loop round record",
    )
}

fn write_loop_toml<T: serde::Serialize + ?Sized>(
    config: &Config,
    path: &Path,
    value: &T,
    op: WriteOp,
    missing_parent_message: &str,
    serialize_message: &str,
) -> DiagnosticResult<()> {
    let parent = path.parent().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            missing_parent_message,
            path.display().to_string(),
        )
    })?;
    let body = toml::to_string_pretty(value).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            format!("{serialize_message}: {e}"),
            path.display().to_string(),
        )
    })?;
    let display_parent = config.display_path(parent);
    let display_path = config.display_path(path);
    crate::write::create_dir_all(parent, op, Some(&display_parent))?;
    crate::write::write_file(path, &body, op, Some(&display_path))?;
    Ok(())
}

fn project_root(config: &Config) -> &Path {
    config
        .gov_root
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}
