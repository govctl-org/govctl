use crate::diagnostic::Diagnostic;
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
    io::stdout()
        .flush()
        .map_err(|err| Diagnostic::io_error("flush confirmation prompt", err, "stdout"))?;

    let mut response = String::new();
    io::stdin()
        .read_line(&mut response)
        .map_err(|err| Diagnostic::io_error("read confirmation response", err, "stdin"))?;

    if !response.trim().eq_ignore_ascii_case("y") {
        ui::info(cancellation_message);
        return Ok(false);
    }

    Ok(true)
}
