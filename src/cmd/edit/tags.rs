use super::engine as edit_engine;
use crate::config::Config;
use crate::diagnostic::DiagnosticResult;

pub(super) fn validate_tag_edit(
    config: &Config,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    let path = match target {
        edit_engine::ResolvedTarget::Node { path, .. } => path,
        edit_engine::ResolvedTarget::IndexedItem { container_path, .. } => container_path,
    };
    if path.as_simple() == Some("tags") {
        crate::cmd::tag::validate_registered_tag(config, value, id)?;
    }
    Ok(())
}
