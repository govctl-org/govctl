//! Render command implementation.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_rfcs;
use crate::render::write_rfc;

/// Render RFC markdown from JSON source
pub fn render(
    config: &Config,
    rfc_id: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfcs = load_rfcs(config).map_err(|e| {
        let diag: Diagnostic = e.into();
        anyhow::anyhow!("{}", diag)
    })?;

    if rfcs.is_empty() {
        eprintln!("No RFCs found in {}", config.rfcs_dir().display());
        return Ok(vec![]);
    }

    // Filter to specific RFC if provided
    let rfcs_to_render: Vec<_> = if let Some(id) = rfc_id {
        rfcs.into_iter().filter(|r| r.rfc.rfc_id == id).collect()
    } else {
        rfcs
    };

    if rfcs_to_render.is_empty() {
        if let Some(id) = rfc_id {
            anyhow::bail!("RFC not found: {id}");
        }
    }

    for rfc in &rfcs_to_render {
        write_rfc(config, rfc, dry_run)?;
    }

    if !dry_run {
        eprintln!("✓ Rendered {} RFC(s)", rfcs_to_render.len());
    }

    Ok(vec![])
}
