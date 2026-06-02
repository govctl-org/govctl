use crate::common;
use crate::common::loop_helpers::{
    loop_add_work, loop_id, loop_item_round_count, loop_item_status, loop_item_table,
    loop_remove_work, loop_replan, loop_resolved, loop_run_with_max_rounds, loop_start_with_id,
    loop_work, work_add_acceptance, work_add_dependency, work_new, work_remove_dependency,
};
use crate::common::{init_project_with_date, run_dynamic_commands};
use std::fs;

#[test]
fn test_loop_add_remove_work_field_accepts_wi_alias_and_rejects_unknown_field() -> common::TestResult
{
    let (temp_dir, date) = init_project_with_date()?;
    let root_id = format!("WI-{date}-001");
    let extra_id = format!("WI-{date}-002");
    let loop_id = loop_id(&date, 1);

    let setup_output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Root"),
            loop_start_with_id(&loop_id, &[&root_id]),
            work_new("Extra"),
        ],
    )?;
    assert!(setup_output.contains("exit: 0"), "{setup_output}");

    let work_items_output = run_dynamic_commands(
        temp_dir.path(),
        &[loop_add_work(&loop_id, "work_items", &extra_id)],
    )?;

    assert!(
        work_items_output.contains("error[E0803]"),
        "{work_items_output}"
    );
    assert!(
        work_items_output.contains("Unknown loop field: work_items"),
        "{work_items_output}"
    );

    let root_work_items_output = run_dynamic_commands(
        temp_dir.path(),
        &[loop_add_work(&loop_id, "root_work_items", &extra_id)],
    )?;

    assert!(
        root_work_items_output.contains("error[E0803]"),
        "{root_work_items_output}"
    );
    assert!(
        root_work_items_output.contains("Unknown loop field: root_work_items"),
        "{root_work_items_output}"
    );

    let wi_output =
        run_dynamic_commands(temp_dir.path(), &[loop_add_work(&loop_id, "wi", &extra_id)])?;

    assert!(
        wi_output.contains(&format!("Updated loop {loop_id}")),
        "{wi_output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(
        loop_work(&toml::from_str(&state_toml)?)?,
        vec![root_id.clone(), extra_id.clone()]
    );

    let wi_remove_output = run_dynamic_commands(
        temp_dir.path(),
        &[loop_remove_work(&loop_id, "wi", &extra_id)],
    )?;

    assert!(
        wi_remove_output.contains(&format!("Updated loop {loop_id}")),
        "{wi_remove_output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_work(&toml::from_str(&state_toml)?)?, vec![root_id]);
    Ok(())
}

#[test]
fn test_loop_root_aliases_are_not_supported() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let loop_id = loop_id(&date, 1);
    let work_id = format!("WI-{date}-001");

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            vec![
                "loop".into(),
                "add-root".into(),
                loop_id.clone(),
                "work".into(),
                work_id.clone(),
            ],
            vec![
                "loop".into(),
                "remove-root".into(),
                loop_id,
                "work".into(),
                work_id,
            ],
        ],
    )?;

    assert!(output.contains("unrecognized subcommand"), "{output}");
    assert!(output.contains("add-root"), "{output}");
    assert!(output.contains("remove-root"), "{output}");
    Ok(())
}

#[test]
fn test_loop_scope_add_remove_and_replan_preserve_current_state() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;
    let original_id = format!("WI-{date}-001");
    let new_dependency_id = format!("WI-{date}-002");
    let new_root_id = format!("WI-{date}-003");
    let loop_id = loop_id(&date, 1);

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            work_new("Original"),
            work_add_acceptance(&original_id, "add: unfinished"),
            loop_start_with_id(&loop_id, &[&original_id]),
            loop_run_with_max_rounds(&loop_id, "2"),
            work_new("Dependency"),
            work_new("New root"),
            work_add_dependency(&new_root_id, &new_dependency_id),
            loop_add_work(&loop_id, "wi", &new_root_id),
        ],
    )?;

    assert!(
        output.contains(&format!("Updated loop {loop_id}")),
        "{output}"
    );
    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    assert_eq!(loop_item_round_count(&state_toml, &original_id)?, 1);
    assert_eq!(loop_item_status(&state_toml, &original_id)?, "active");
    let state: toml::Value = toml::from_str(&state_toml)?;
    assert_eq!(
        loop_work(&state)?,
        vec![original_id.clone(), new_root_id.clone()]
    );
    assert_eq!(
        loop_resolved(&state)?,
        vec![
            original_id.clone(),
            new_dependency_id.clone(),
            new_root_id.clone()
        ]
    );

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[
            loop_remove_work(&loop_id, "work", &original_id),
            work_remove_dependency(&new_root_id, &new_dependency_id),
            loop_replan(&loop_id),
        ],
    )?;

    assert!(
        output.contains(&format!("Replanned loop {loop_id}")),
        "{output}"
    );

    let state_toml = fs::read_to_string(
        temp_dir
            .path()
            .join(format!(".govctl/loops/{loop_id}/state.toml")),
    )?;
    let state: toml::Value = toml::from_str(&state_toml)?;
    assert_eq!(loop_work(&state)?, vec![new_root_id.clone()]);
    assert_eq!(loop_resolved(&state)?, vec![new_root_id.clone()]);
    assert!(
        loop_item_table(&state, &original_id).is_err(),
        "removed root should no longer have current item state: {state_toml}"
    );
    assert!(
        loop_item_table(&state, &new_dependency_id).is_err(),
        "replan should remove dependencies no longer needed: {state_toml}"
    );
    Ok(())
}
