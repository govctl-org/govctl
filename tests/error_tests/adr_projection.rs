use super::*;

fn write_adr_with_title(
    dir: &std::path::Path,
    status: &str,
    title: &str,
    context: &str,
) -> common::TestResult {
    fs::write(
        dir.join("gov/adr/ADR-0001-projection.toml"),
        format!(
            r#"[govctl]
id = "ADR-0001"
title = "{title}"
status = "{status}"
date = "2026-01-01"

[content]
context = '''{context}'''
decision = "Use structured alternatives."
consequences = "Projection remains deterministic."

[[content.alternatives]]
text = "Option A"
status = "accepted"
"#
        ),
    )?;
    Ok(())
}

fn write_adr(dir: &std::path::Path, status: &str, context: &str) -> common::TestResult {
    write_adr_with_title(dir, status, "Projection Test", context)
}

#[test]
fn test_check_rejects_proposed_adr_projection_headings() -> common::TestResult {
    let temp_dir = init_project()?;
    write_adr(
        temp_dir.path(),
        "proposed",
        "### **Options** `Considered`\n\n### Option A (accepted)",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert_eq!(output.matches("error[E0307]").count(), 2, "{output}");
    assert!(output.contains("content.context"), "{output}");
    assert!(output.contains("heading 'Options Considered'"), "{output}");
    assert!(output.contains("heading 'Option A (accepted)'"), "{output}");
    Ok(())
}

#[test]
fn test_check_ignores_projection_heading_inside_fenced_code() -> common::TestResult {
    let temp_dir = init_project()?;
    write_adr(temp_dir.path(), "proposed", "```markdown\n## Decision\n```")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(!output.contains("error[E0307]"), "{output}");
    assert!(output.contains("exit: 0"), "{output}");
    Ok(())
}

#[test]
fn test_check_compares_formatted_title_by_visible_text() -> common::TestResult {
    let temp_dir = init_project()?;
    write_adr_with_title(
        temp_dir.path(),
        "proposed",
        "**Projection** `Test`",
        "# ADR-0001: Projection Test",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(output.contains("error[E0307]"), "{output}");
    assert!(
        output.contains("heading 'ADR-0001: Projection Test'"),
        "{output}"
    );
    Ok(())
}

#[test]
fn test_check_preserves_historical_adr_compatibility() -> common::TestResult {
    let temp_dir = init_project()?;
    write_adr(temp_dir.path(), "accepted", "### Options Considered")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(!output.contains("error[E0307]"), "{output}");
    assert!(output.contains("exit: 0"), "{output}");
    Ok(())
}
