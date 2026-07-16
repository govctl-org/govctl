use super::*;

fn rfc_dir_for(project_dir: &std::path::Path, rfc_id: &str) -> std::path::PathBuf {
    project_dir.join("gov/rfc").join(rfc_id)
}

fn write_rfc_toml(project_dir: &std::path::Path, content: &str) -> common::TestResult {
    write_rfc_toml_for(project_dir, "RFC-0001", content)
}

fn write_rfc_toml_for(
    project_dir: &std::path::Path,
    rfc_id: &str,
    content: &str,
) -> common::TestResult {
    let rfc_dir = rfc_dir_for(project_dir, rfc_id);
    fs::create_dir_all(rfc_dir.join("clauses"))?;
    fs::write(rfc_dir.join("rfc.toml"), content)?;
    Ok(())
}

fn write_clause_toml(
    project_dir: &std::path::Path,
    file_name: &str,
    content: &str,
) -> common::TestResult {
    write_clause_toml_for(project_dir, "RFC-0001", file_name, content)
}

fn write_clause_toml_for(
    project_dir: &std::path::Path,
    rfc_id: &str,
    file_name: &str,
    content: &str,
) -> common::TestResult {
    let rfc_dir = rfc_dir_for(project_dir, rfc_id);
    fs::create_dir_all(rfc_dir.join("clauses"))?;
    fs::write(rfc_dir.join("clauses").join(file_name), content)?;
    Ok(())
}

fn write_adr_toml(project_dir: &std::path::Path, content: &str) -> common::TestResult {
    fs::write(
        project_dir.join("gov/adr/ADR-0001-lower-authority.toml"),
        content,
    )?;
    Ok(())
}

#[test]
fn test_rfc_check_rejects_missing_current_changelog_entry() -> common::TestResult {
    let temp_dir = init_project()?;
    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
id = "RFC-0001"
title = "Missing Current Changelog"
version = "1.0.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"

[[sections]]
title = "Specification"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0111]"), "output: {output}");
    assert!(output.contains("found 0"), "output: {output}");
    Ok(())
}

#[test]
fn test_rfc_check_rejects_duplicate_current_changelog_entries() -> common::TestResult {
    let temp_dir = init_project()?;
    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
id = "RFC-0001"
title = "Duplicate Current Changelog"
version = "1.0.0"
status = "draft"
phase = "spec"
owners = ["@test"]
created = "2026-01-01"

[[sections]]
title = "Specification"

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
notes = "First"

[[changelog]]
version = "1.0.0"
date = "2026-01-02"
notes = "Duplicate"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0115]"), "output: {output}");
    assert!(output.contains("found 2"), "output: {output}");
    Ok(())
}

#[test]
fn test_broken_superseded_check() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Broken Superseded Test"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Clauses"
clauses = ["clauses/C-OLD.toml", "clauses/C-NEW.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-OLD.toml",
        r#"[govctl]
schema = 1
id = "C-OLD"
title = "Old Clause"
kind = "normative"
status = "superseded"
superseded_by = "C-NONEXISTENT"
since = "1.0.0"

[content]
text = "This clause is superseded."
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-NEW.toml",
        r#"[govctl]
schema = 1
id = "C-NEW"
title = "New Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "This is the new clause."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_superseded_clause_without_replacement_check() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Missing Replacement Test"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Clauses"
clauses = ["clauses/C-OLD.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-OLD.toml",
        r#"[govctl]
schema = 1
id = "C-OLD"
title = "Old Clause"
kind = "normative"
status = "superseded"
since = "1.0.0"

[content]
text = "This clause is superseded."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(
        output.contains("error[E0213]: Superseded clause 'C-OLD' has no superseded_by target"),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 1\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_clause_supersession_cycle_check() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Supersession Cycle Test"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Clauses"
clauses = ["clauses/C-ONE.toml", "clauses/C-TWO.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-ONE.toml",
        r#"[govctl]
schema = 1
id = "C-ONE"
title = "Clause One"
kind = "normative"
status = "superseded"
superseded_by = "C-TWO"
since = "1.0.0"

[content]
text = "Clause one."
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TWO.toml",
        r#"[govctl]
schema = 1
id = "C-TWO"
title = "Clause Two"
kind = "normative"
status = "superseded"
superseded_by = "C-ONE"
since = "1.0.0"

[content]
text = "Clause two."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0212]"), "output: {output}");
    assert!(
        output.contains("Clause supersession cycle detected"),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 1\n\n"), "output: {output}");
    Ok(())
}

#[test]
fn test_cross_rfc_clause_supersession_cycle_check() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml_for(
        temp_dir.path(),
        "RFC-0001",
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "First RFC"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Clauses"
clauses = ["clauses/C-ONE.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;
    write_rfc_toml_for(
        temp_dir.path(),
        "RFC-0002",
        r#"[govctl]
schema = 1
id = "RFC-0002"
title = "Second RFC"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Clauses"
clauses = ["clauses/C-TWO.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;
    write_clause_toml_for(
        temp_dir.path(),
        "RFC-0001",
        "C-ONE.toml",
        r#"[govctl]
schema = 1
id = "C-ONE"
title = "Clause One"
kind = "normative"
status = "superseded"
superseded_by = "RFC-0002:C-TWO"
since = "1.0.0"

[content]
text = "Clause one."
"#,
    )?;
    write_clause_toml_for(
        temp_dir.path(),
        "RFC-0002",
        "C-TWO.toml",
        r#"[govctl]
schema = 1
id = "C-TWO"
title = "Clause Two"
kind = "normative"
status = "superseded"
superseded_by = "RFC-0001:C-ONE"
since = "1.0.0"

[content]
text = "Clause two."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0212]"), "output: {output}");
    assert!(
        output.contains(
            "Clause supersession cycle detected: RFC-0001:C-ONE -> RFC-0002:C-TWO -> RFC-0001:C-ONE"
        ),
        "output: {output}"
    );
    assert!(output.ends_with("exit: 1\n\n"), "output: {output}");
    Ok(())
}

/// Test: RFC has invalid status/phase combination (draft + stable)
#[test]
fn test_invalid_transition_check() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Invalid Transition Test"
version = "0.1.0"
status = "draft"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
added = ["Initial draft"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "0.1.0"

[content]
text = "A test clause in an invalid RFC."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_error_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_rfc_plain_text_adr_reference_violates_hierarchy() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Plain ADR Reference"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "This RFC tries to cite ADR-0001 without brackets."
"#,
    )?;

    write_adr_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Lower Authority"
status = "accepted"
date = "2026-01-01"
refs = ["RFC-0001"]

[content]
context = "Context"
decision = "Decision"
consequences = "Consequences"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0112]"), "output: {}", output);
    assert!(output.contains("mentions ADR-0001"), "output: {}", output);
    Ok(())
}

#[test]
fn test_rfc_plain_text_nonexistent_adr_is_allowed() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Nonexistent ADR Mention"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "This RFC mentions ADR-9999 as a nonexistent example."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(!output.contains("error[E0112]"), "output: {}", output);
    assert!(!output.contains("error[E0306]"), "output: {}", output);
    assert!(!output.contains("mentions ADR-9999"), "output: {}", output);
    Ok(())
}

#[test]
fn test_proposed_adr_plain_text_known_rfc_reference_warns() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Known RFC"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "Specification."
"#,
    )?;

    write_adr_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Decision"
status = "proposed"
date = "2026-01-01"
refs = ["RFC-0001"]

[content]
context = "Context"
decision = "This decision follows RFC-0001."
consequences = "Consequences"
"#,
    )?;

    let output = run_commands(
        temp_dir.path(),
        &[&["check"], &["check", "--deny-warnings"]],
    )?;
    assert!(output.contains("warning[W0112]"), "output: {}", output);
    assert!(
        output.contains("content.decision line 1"),
        "output: {}",
        output
    );
    assert!(
        output.contains("context: \"This decision follows RFC-0001.\""),
        "output: {}",
        output
    );
    assert!(output.contains("use [[RFC-0001]]"), "output: {}", output);
    assert!(output.contains("$ govctl check\n"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);
    assert!(
        output.contains("$ govctl check --deny-warnings\n"),
        "output: {}",
        output
    );
    assert!(output.contains("exit: 1"), "output: {}", output);
    Ok(())
}

#[test]
fn test_proposed_adr_alternative_plain_text_known_rfc_references_warn() -> common::TestResult {
    let temp_dir = init_project()?;
    write_minimal_rfc(temp_dir.path(), "RFC-0001", "Known RFC")?;

    write_adr_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Decision"
status = "proposed"
date = "2026-01-01"
refs = ["RFC-0001"]

[content]
context = "Context"
decision = "Decision"
consequences = "Consequences"

[[content.alternatives]]
text = "This alternative follows RFC-0001."
pros = ["RFC-0001 gives this option clear authority."]
cons = ["RFC-0001 adds migration pressure."]
rejection_reason = "Rejected after comparing with RFC-0001."
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_eq!(
        output.matches("warning[W0112]").count(),
        4,
        "output: {}",
        output
    );
    assert!(output.contains("Artifact 'ADR-0001'"), "output: {}", output);
    assert!(
        output.contains("content.alternatives[0].text line 1"),
        "output: {}",
        output
    );
    assert!(
        output.contains("content.alternatives[0].pros[0] line 1"),
        "output: {}",
        output
    );
    assert!(
        output.contains("content.alternatives[0].cons[0] line 1"),
        "output: {}",
        output
    );
    assert!(
        output.contains("content.alternatives[0].rejection_reason line 1"),
        "output: {}",
        output
    );
    assert!(output.contains("exit: 0"), "output: {}", output);
    Ok(())
}

#[test]
fn test_adr_bracketed_known_rfc_reference_does_not_warn() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Known RFC"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "Specification."
"#,
    )?;

    write_adr_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Decision"
status = "accepted"
date = "2026-01-01"
refs = ["RFC-0001"]

[content]
context = "Context"
decision = "This decision follows [[RFC-0001]]."
consequences = "Consequences"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(!output.contains("warning[W0112]"), "output: {}", output);
    assert!(output.contains("exit: 0"), "output: {}", output);
    Ok(())
}

#[test]
fn test_rfc_changelog_plain_text_adr_reference_is_allowed() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Changelog ADR Mention"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
notes = "This release note mentions ADR-0001 as prose."
added = ["Initial release mentions ADR-0001 as prose"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "This RFC clause does not cite lower authority artifacts."
"#,
    )?;

    write_adr_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Lower Authority"
status = "accepted"
date = "2026-01-01"
refs = ["RFC-0001"]

[content]
context = "Context"
decision = "Decision"
consequences = "Consequences"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(!output.contains("error[E0112]"), "output: {}", output);
    Ok(())
}

#[test]
fn test_rfc_bracketed_adr_reference_reports_once() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Bracket ADR Reference"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
clauses = ["clauses/C-TEST.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    write_clause_toml(
        temp_dir.path(),
        "C-TEST.toml",
        r#"[govctl]
schema = 1
id = "C-TEST"
title = "Test Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "This RFC links to [[ADR-0001]]."
"#,
    )?;

    write_adr_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Lower Authority"
status = "accepted"
date = "2026-01-01"
refs = ["RFC-0001"]

[content]
context = "Context"
decision = "Decision"
consequences = "Consequences"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_eq!(
        output.matches("error[E0112]").count(),
        1,
        "output: {output}"
    );
    assert_eq!(
        output.matches("links to [[ADR-0001]]").count(),
        1,
        "output: {output}"
    );
    Ok(())
}

#[test]
fn test_adr_plain_text_work_reference_violates_hierarchy() -> common::TestResult {
    let temp_dir = init_project()?;

    write_rfc_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Governing RFC"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Overview"
"#,
    )?;

    write_adr_toml(
        temp_dir.path(),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Plain Work Reference"
status = "accepted"
date = "2026-01-01"
refs = ["RFC-0001"]

[content]
context = "Context"
decision = "This ADR tries to cite WI-2026-01-01-001 without brackets."
consequences = "Consequences"
"#,
    )?;

    fs::write(
        temp_dir.path().join("gov/work/2026-01-01-task.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Execution task"
status = "queue"
created = "2026-01-01"

[content]
description = "Work description"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0306]"), "output: {}", output);
    assert!(
        output.contains("mentions WI-2026-01-01-001"),
        "output: {}",
        output
    );
    Ok(())
}
