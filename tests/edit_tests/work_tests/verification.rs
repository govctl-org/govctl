use super::*;

const REQUIRED_GUARDS: &str = "verification.required_guards";

fn work_edit_add_required_guard(id: &str, guard: &str) -> Vec<String> {
    command(&["work", "edit", id, REQUIRED_GUARDS, "--add", guard])
}

fn work_edit_remove_required_guard(id: &str, guard: &str) -> Vec<String> {
    command(&["work", "edit", id, REQUIRED_GUARDS, "--remove", guard])
}

fn work_edit_set_waiver_reason(id: &str, index: usize, reason: &str) -> Vec<String> {
    let field = format!("verification.waivers[{index}].reason");
    command(&["work", "edit", id, &field, "--set", reason])
}

fn work_edit_set_waiver_guard(id: &str, index: usize, guard: &str) -> Vec<String> {
    let field = format!("verification.waivers[{index}].guard");
    command(&["work", "edit", id, &field, "--set", guard])
}

#[test]
fn test_work_edit_required_guards_add_get_remove() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let id = first_work_id(&date);
    common::write_guard(temp_dir.path(), "GUARD-EDIT", "true")?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Guarded Task"),
            work_edit_add_required_guard(&id, "GUARD-EDIT"),
            work_get_field(&id, REQUIRED_GUARDS),
            work_edit_remove_required_guard(&id, "GUARD-EDIT"),
            work_get_field(&id, REQUIRED_GUARDS),
        ],
    )?;

    assert!(
        output.contains(&format!(
            "Added 'GUARD-EDIT' to {id}.verification.required_guards"
        )),
        "output: {}",
        output
    );
    assert!(
        output.contains(&format!(
            "$ govctl work get {id} verification.required_guards\nGUARD-EDIT"
        )),
        "output: {}",
        output
    );
    assert!(
        output.contains(&format!(
            "Removed 'GUARD-EDIT' from {id}.verification.required_guards"
        )),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_work_edit_existing_waiver_guard_and_reason() -> common::TestResult {
    let temp_dir = init_project()?;
    common::write_guard(temp_dir.path(), "GUARD-WAIVED", "true")?;
    common::write_guarded_work_item(
        temp_dir.path(),
        "WI-2026-01-01-001",
        "GUARD-WAIVED",
        Some("Initial reason"),
    )?;

    let output = common::run_dynamic_commands(
        temp_dir.path(),
        &[
            work_get_field("WI-2026-01-01-001", "verification.waivers[0].guard"),
            work_get_field("WI-2026-01-01-001", "verification.waivers[0].reason"),
            work_edit_set_waiver_guard("WI-2026-01-01-001", 0, "GUARD-WAIVED"),
            work_edit_set_waiver_reason("WI-2026-01-01-001", 0, "Updated waiver reason"),
            work_get_field("WI-2026-01-01-001", "verification.waivers[0].guard"),
            work_get_field("WI-2026-01-01-001", "verification.waivers[0].reason"),
        ],
    )?;

    assert!(output.contains("GUARD-WAIVED"), "output: {}", output);
    assert!(output.contains("Initial reason"), "output: {}", output);
    assert!(
        output.contains("Set WI-2026-01-01-001.verification.waivers[0].guard = GUARD-WAIVED"),
        "output: {}",
        output
    );
    assert!(
        output.contains(
            "Set WI-2026-01-01-001.verification.waivers[0].reason = Updated waiver reason"
        ),
        "output: {}",
        output
    );
    assert!(
        output.contains("Updated waiver reason"),
        "output: {}",
        output
    );
    Ok(())
}
