use super::*;

#[test]
fn test_work_legacy_inline_history_reports_info_and_passes_check() -> common::TestResult {
    let temp_dir = init_project()?;

    fs::write(
        temp_dir
            .path()
            .join("gov/work/2026-01-01-legacy-history.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Legacy History"
status = "queue"
created = "2026-01-01"

[content]
description = "Work description"

[[content.journal]]
date = "2026-01-01"
content = "Historical execution detail"
"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["check"], &["check", "--deny-warnings"]],
    )?;
    assert!(output.contains("info[I0401]"), "output: {}", output);
    assert!(
        output.contains("legacy inline execution history"),
        "output: {}",
        output
    );
    assert!(output.contains("notes"), "output: {}", output);
    assert!(output.contains("loop state"), "output: {}", output);
    assert!(output.contains("✓ All checks passed"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(!output.contains("exit: 1"), "output: {}", output);
    Ok(())
}

/// Test: Work item files without legacy inline execution history do not report info
#[test]
fn test_work_without_legacy_inline_history_has_no_info() -> common::TestResult {
    let temp_dir = init_project()?;

    fs::write(
        temp_dir.path().join("gov/work/2026-01-01-normal.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Normal Work"
status = "queue"
created = "2026-01-01"

[content]
description = "Work description"

[[content.acceptance_criteria]]
text = "Criterion"
status = "pending"
category = "added"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(!output.contains("info[I0401]"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);
    Ok(())
}

#[test]
fn test_work_plain_text_known_rfc_reference_warns() -> common::TestResult {
    let temp_dir = init_project()?;
    write_minimal_rfc(temp_dir.path(), "RFC-0001", "Known RFC")?;

    fs::write(
        temp_dir
            .path()
            .join("gov/work/2026-01-01-bare-reference.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Bare Reference"
status = "queue"
created = "2026-01-01"
refs = ["RFC-0001"]

[content]
description = "This work follows RFC-0001."

[[content.acceptance_criteria]]
text = "Use [[RFC-0001]] in bracketed form here"
status = "pending"
category = "chore"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("warning[W0112]"), "output: {}", output);
    assert!(
        output.contains("Artifact 'WI-2026-01-01-001' mentions known artifact ID RFC-0001"),
        "output: {}",
        output
    );
    assert!(output.contains("use [[RFC-0001]]"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);
    Ok(())
}

#[test]
fn test_work_acceptance_and_notes_plain_text_known_rfc_reference_warns() -> common::TestResult {
    let temp_dir = init_project()?;
    write_minimal_rfc(temp_dir.path(), "RFC-0001", "Known RFC")?;

    fs::write(
        temp_dir
            .path()
            .join("gov/work/2026-01-01-bare-reference-fields.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Bare Reference Fields"
status = "active"
created = "2026-01-01"
started = "2026-01-01"
refs = ["RFC-0001"]

[content]
description = "Work description."
notes = ["Keep RFC-0001 in mind for closure."]

[[content.acceptance_criteria]]
text = "Complete the RFC-0001 follow-up"
status = "pending"
category = "chore"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_eq!(
        output.matches("warning[W0112]").count(),
        2,
        "output: {}",
        output
    );
    assert!(
        output.contains("Artifact 'WI-2026-01-01-001' mentions known artifact ID RFC-0001"),
        "output: {}",
        output
    );
    assert!(output.contains("exit: 0"), "output: {}", output);
    Ok(())
}

#[test]
fn test_done_work_plain_text_known_rfc_reference_is_allowed() -> common::TestResult {
    let temp_dir = init_project()?;
    write_minimal_rfc(temp_dir.path(), "RFC-0001", "Known RFC")?;

    fs::write(
        temp_dir
            .path()
            .join("gov/work/2026-01-01-done-bare-reference.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Done Bare Reference"
status = "done"
created = "2026-01-01"
started = "2026-01-01"
completed = "2026-01-01"
refs = ["RFC-0001"]

[content]
description = "This completed work followed RFC-0001."
notes = ["RFC-0001 remains useful historical context."]

[[content.acceptance_criteria]]
text = "Completed the RFC-0001 follow-up"
status = "done"
category = "chore"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(!output.contains("warning[W0112]"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);
    Ok(())
}

#[test]
fn test_check_rejects_unknown_work_dependency() -> common::TestResult {
    let temp_dir = init_project()?;

    fs::write(
        temp_dir
            .path()
            .join("gov/work/2026-01-01-unknown-dependency.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Unknown Dependency"
status = "queue"
created = "2026-01-01"
depends_on = ["WI-2026-01-01-999"]

[content]
description = "Work description"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0410]"), "output: {}", output);
    assert!(
        output.contains("unknown work item dependency"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_check_rejects_work_dependency_cycle() -> common::TestResult {
    let temp_dir = init_project()?;

    fs::write(
        temp_dir.path().join("gov/work/2026-01-01-cycle-a.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Cycle A"
status = "queue"
created = "2026-01-01"
depends_on = ["WI-2026-01-01-002"]

[content]
description = "Work description"
"#,
    )?;
    fs::write(
        temp_dir.path().join("gov/work/2026-01-01-cycle-b.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-002"
title = "Cycle B"
status = "queue"
created = "2026-01-01"
depends_on = ["WI-2026-01-01-001"]

[content]
description = "Work description"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0411]"), "output: {}", output);
    assert!(
        output.contains("cyclic work item dependency"),
        "output: {}",
        output
    );
    assert!(
        output.contains("WI-2026-01-01-001") || output.contains("WI-2026-01-01-002"),
        "output: {}",
        output
    );
    Ok(())
}
