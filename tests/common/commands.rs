use super::normalize_output;
use std::path::Path;
use std::process::Command;

pub fn command(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}

pub fn work_new(title: &str) -> Vec<String> {
    command(&["work", "new", title])
}

pub fn work_new_active(title: &str) -> Vec<String> {
    let mut cmd = work_new(title);
    cmd.push("--active".to_string());
    cmd
}

pub fn work_show(work_id: &str) -> Vec<String> {
    command(&["work", "show", work_id])
}

pub fn work_list_all() -> Vec<String> {
    command(&["work", "list", "all"])
}

pub fn work_get_field(work_id: &str, field: &str) -> Vec<String> {
    command(&["work", "get", work_id, field])
}

pub fn work_set_field(work_id: &str, field: &str, value: &str) -> Vec<String> {
    command(&["work", "set", work_id, field, value])
}

pub fn work_add_field(work_id: &str, field: &str, value: &str) -> Vec<String> {
    command(&["work", "add", work_id, field, value])
}

pub fn work_remove_field(work_id: &str, field: &str, value: &str) -> Vec<String> {
    command(&["work", "remove", work_id, field, value])
}

pub fn work_add_acceptance(work_id: &str, text: &str) -> Vec<String> {
    work_add_field(work_id, "acceptance_criteria", text)
}

pub fn work_tick_acceptance(work_id: &str, pattern: &str, status: &str) -> Vec<String> {
    command(&[
        "work",
        "tick",
        work_id,
        "acceptance_criteria",
        pattern,
        "-s",
        status,
    ])
}

pub fn work_tick_acceptance_done(work_id: &str, pattern: &str) -> Vec<String> {
    work_tick_acceptance(work_id, pattern, "done")
}

pub fn work_remove_acceptance(work_id: &str, pattern: &str) -> Vec<String> {
    work_remove_field(work_id, "acceptance_criteria", pattern)
}

pub fn work_move_done(work_id: &str) -> Vec<String> {
    command(&["work", "move", work_id, "done"])
}

pub fn work_delete_force(work_id: &str) -> Vec<String> {
    command(&["work", "delete", work_id, "-f"])
}

pub fn work_add_dependency(work_id: &str, dependency_id: &str) -> Vec<String> {
    work_add_field(work_id, "depends_on", dependency_id)
}

pub fn work_remove_dependency(work_id: &str, dependency_id: &str) -> Vec<String> {
    work_remove_field(work_id, "depends_on", dependency_id)
}

pub fn run_normalized_commands(
    dir: &Path,
    date: &str,
    commands: &[&[&str]],
) -> Result<String, Box<dyn std::error::Error>> {
    let output = run_commands(dir, commands)?;
    Ok(normalize_output(&output, dir, date)?)
}

/// Run govctl commands in a directory and capture output.
pub fn run_commands(dir: &Path, commands: &[&[&str]]) -> Result<String, std::io::Error> {
    let mut output = String::new();

    for args in commands {
        append_command_output(dir, args, &mut output)?;
    }

    Ok(output)
}

/// Run commands with dynamic String arguments (for work item IDs with dates)
pub fn run_dynamic_commands(
    dir: &Path,
    commands: &[Vec<String>],
) -> Result<String, std::io::Error> {
    let mut output = String::new();

    for args in commands {
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        append_command_output(dir, &args_str, &mut output)?;
    }

    Ok(output)
}

fn append_command_output(
    dir: &Path,
    args: &[&str],
    output: &mut String,
) -> Result<(), std::io::Error> {
    let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .output()?;

    output.push_str(&format_command_output(args, &result));

    Ok(())
}

pub fn format_command_output(args: &[&str], result: &std::process::Output) -> String {
    let mut output = format!("$ govctl {}\n", args.join(" "));
    append_process_output(&mut output, &result.stdout);
    append_process_output(&mut output, &result.stderr);
    output.push_str(&format!("exit: {}\n\n", result.status.code().unwrap_or(-1)));
    output
}

fn append_process_output(output: &mut String, bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    if !text.is_empty() {
        output.push_str(&text);
        if !text.ends_with('\n') {
            output.push('\n');
        }
    }
}
