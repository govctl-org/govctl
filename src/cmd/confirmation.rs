use crate::ui;
use crate::write::WriteOp;
use std::io::{self, Write};

pub(crate) fn confirm_destructive_action(
    force: bool,
    op: WriteOp,
    prompt: &str,
    cancellation_message: &str,
) -> anyhow::Result<bool> {
    if force || op.is_preview() {
        return Ok(true);
    }

    print!("{prompt} [y/N] ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    if !response.trim().eq_ignore_ascii_case("y") {
        ui::info(cancellation_message);
        return Ok(false);
    }

    Ok(true)
}
