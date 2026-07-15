use super::*;

fn write_done_work_item(dir: &std::path::Path) -> common::TestResult {
    fs::write(
        dir.join("gov/work/2026-01-01-done.toml"),
        r#"[govctl]
id = "WI-2026-01-01-001"
title = "Done work"
status = "done"
created = "2026-01-01"
started = "2026-01-01"
completed = "2026-01-01"

[content]
description = "Completed work"

[[content.acceptance_criteria]]
text = "Done"
status = "done"
category = "chore"
"#,
    )?;
    Ok(())
}

#[test]
fn test_check_rejects_work_item_referenced_by_multiple_releases() -> common::TestResult {
    let temp_dir = init_project()?;
    write_done_work_item(temp_dir.path())?;
    fs::write(
        temp_dir.path().join("gov/releases.toml"),
        r#"[[releases]]
version = "0.2.0"
date = "2026-01-02"
refs = ["WI-2026-01-01-001"]

[[releases]]
version = "0.1.0"
date = "2026-01-01"
refs = ["WI-2026-01-01-001"]
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0707]"), "output: {output}");
    assert!(
        output.contains("referenced by releases '0.2.0' and '0.1.0'"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_check_rejects_released_work_item_that_is_not_done() -> common::TestResult {
    let temp_dir = init_project()?;
    fs::write(
        temp_dir.path().join("gov/work/2026-01-01-active.toml"),
        r#"[govctl]
id = "WI-2026-01-01-001"
title = "Active work"
status = "active"
created = "2026-01-01"
started = "2026-01-01"

[content]
description = "Active work"
"#,
    )?;
    fs::write(
        temp_dir.path().join("gov/releases.toml"),
        r#"[[releases]]
version = "0.1.0"
date = "2026-01-01"
refs = ["WI-2026-01-01-001"]
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0706]"), "output: {output}");
    assert!(
        output.contains("status other than done"),
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_release_rejects_invalid_calendar_dates_without_writing() -> common::TestResult {
    let temp_dir = init_project()?;
    write_done_work_item(temp_dir.path())?;
    let releases_path = temp_dir.path().join("gov/releases.toml");

    let output = run_commands(
        temp_dir.path(),
        &[
            &["release", "0.1.0", "--date", "2026/01/01"],
            &["release", "0.1.0", "--date", "2026-02-30"],
        ],
    )?;

    assert_eq!(output.matches("error[E0704]").count(), 2, "{output}");
    assert!(!releases_path.exists(), "invalid release created a file");
    Ok(())
}

#[test]
fn test_check_rejects_release_entry_without_refs() -> common::TestResult {
    let temp_dir = init_project()?;
    fs::write(
        temp_dir.path().join("gov/releases.toml"),
        r#"[[releases]]
version = "0.1.0"
date = "2026-01-01"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0704]"), "output: {output}");
    assert!(
        output.contains("\"refs\" is a required property"),
        "{output}"
    );
    Ok(())
}

#[test]
fn test_release_undo_rejects_empty_history() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["release", "undo", "0.1.0"]])?;

    assert!(output.contains("error[E0708]"), "output: {output}");
    assert!(
        output.contains("release history is empty"),
        "output: {output}"
    );
    assert!(!temp_dir.path().join("gov/releases.toml").exists());
    Ok(())
}

#[test]
fn test_release_undo_rejects_invalid_expected_version() -> common::TestResult {
    let temp_dir = init_project()?;

    let output = run_commands(temp_dir.path(), &[&["release", "undo", "not-semver"]])?;

    assert!(output.contains("error[E0701]"), "output: {output}");
    assert!(!temp_dir.path().join("gov/releases.toml").exists());
    Ok(())
}

#[test]
fn test_release_undo_rejects_stale_expected_version_without_writing() -> common::TestResult {
    let temp_dir = init_project()?;
    let releases_path = temp_dir.path().join("gov/releases.toml");
    let releases = r#"[[releases]]
version = "0.2.0"
date = "2026-01-02"
refs = ["WI-2026-01-02-001"]

[[releases]]
version = "0.1.0"
date = "2026-01-01"
refs = ["WI-2026-01-01-001"]
"#;
    fs::write(&releases_path, releases)?;

    let output = run_commands(temp_dir.path(), &[&["release", "undo", "0.1.0"]])?;

    assert!(output.contains("error[E0709]"), "output: {output}");
    assert!(
        output.contains("newest release is 0.2.0"),
        "output: {output}"
    );
    assert_eq!(fs::read_to_string(releases_path)?, releases);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_release_undo_write_failure_preserves_release_bytes() -> common::TestResult {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = init_project()?;
    let gov_dir = temp_dir.path().join("gov");
    let releases_path = gov_dir.join("releases.toml");
    let releases = r#"[[releases]]
version = "0.2.0"
date = "2026-01-02"
refs = ["WI-2026-01-02-001"]

[[releases]]
version = "0.1.0"
date = "2026-01-01"
refs = ["WI-2026-01-01-001"]
"#;
    fs::write(&releases_path, releases)?;

    let original_mode = fs::metadata(&gov_dir)?.permissions().mode();
    fs::set_permissions(&gov_dir, fs::Permissions::from_mode(0o555))?;
    let output = run_commands(temp_dir.path(), &[&["release", "undo", "0.2.0"]]);
    fs::set_permissions(&gov_dir, fs::Permissions::from_mode(original_mode))?;
    let output = output?;

    assert!(output.contains("error[E0901]"), "output: {output}");
    assert_eq!(fs::read_to_string(releases_path)?, releases);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_release_undo_delete_failure_preserves_only_release() -> common::TestResult {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = init_project()?;
    let gov_dir = temp_dir.path().join("gov");
    let releases_path = gov_dir.join("releases.toml");
    let releases = r#"[[releases]]
version = "0.1.0"
date = "2026-01-01"
refs = ["WI-2026-01-01-001"]
"#;
    fs::write(&releases_path, releases)?;

    let original_mode = fs::metadata(&gov_dir)?.permissions().mode();
    fs::set_permissions(&gov_dir, fs::Permissions::from_mode(0o555))?;
    let output = run_commands(temp_dir.path(), &[&["release", "undo", "0.1.0"]]);
    fs::set_permissions(&gov_dir, fs::Permissions::from_mode(original_mode))?;
    let output = output?;

    assert!(output.contains("error[E0901]"), "output: {output}");
    assert_eq!(fs::read_to_string(releases_path)?, releases);
    Ok(())
}
