mod common;

use common::{init_project, init_project_at, run_commands};
use std::fs;
use std::path::Path;

fn establish_requirement(dir: &Path) -> common::TestResult {
    let output = run_commands(
        dir,
        &[
            &["rfc", "new", "Trace RFC", "--id", "RFC-0001"],
            &["clause", "new", "RFC-0001:C-REQ", "Trace requirement"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["guard", "new", "Trace Guard"],
        ],
    )?;
    assert!(!output.contains("exit: 1"), "{output}");
    fs::write(dir.join("scenario.txt"), "scenario\n")?;
    Ok(())
}

#[test]
fn conformance_crud_trace_search_and_check_form_one_resource_path() -> common::TestResult {
    let temp_dir = init_project()?;
    establish_requirement(temp_dir.path())?;

    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "conformance",
                "new",
                "Trace Case",
                "--path",
                "scenario.txt",
                "--selector",
                "*",
                "--requirement",
                "RFC-0001:C-REQ@0.1.0",
                "--guard",
                "GUARD-TRACE-GUARD",
            ],
            &[
                "conformance",
                "edit",
                "CONF-TRACE-CASE",
                "title",
                "--set",
                "Edited Trace Case",
            ],
            &[
                "conformance",
                "new",
                "Trace Case",
                "--path",
                "scenario.txt",
                "--selector",
                "alternate",
                "--requirement",
                "RFC-0001:C-REQ@0.1.0",
            ],
            &["conformance", "get", "CONF-TRACE-CASE", "requirements"],
            &["conformance", "trace", "CONF-TRACE-CASE", "-o", "table"],
            &["conformance", "trace", "CONF-TRACE-CASE", "-o", "json"],
            &["search", "Edited", "--type", "conformance", "-o", "json"],
            &["check"],
        ],
    )?;

    assert!(!output.contains("exit: 1"), "{output}");
    assert!(output.contains("RFC-0001:C-REQ@0.1.0"), "{output}");
    assert!(
        output.contains("Created Conformance Case: CONF-TRACE-CASE-2"),
        "{output}"
    );
    assert!(output.contains("Edited Trace Case"), "{output}");
    assert!(output.contains("Tags"), "{output}");
    assert!(
        output.contains("\"requirement_applicability\": \"candidate\""),
        "{output}"
    );
    assert!(output.contains("\"kind\": \"conformance\""), "{output}");
    assert!(
        output.contains("\"path\": \"gov/conformance/CONF-TRACE-CASE.toml\""),
        "{output}"
    );
    assert!(
        output.contains("\"scenario_path\": \"scenario.txt\""),
        "{output}"
    );
    assert!(output.contains("2 conformance cases"), "{output}");
    Ok(())
}

#[test]
fn conformance_mutations_validate_the_target_before_writing() -> common::TestResult {
    let temp_dir = init_project()?;
    establish_requirement(temp_dir.path())?;
    let created = run_commands(
        temp_dir.path(),
        &[&[
            "conformance",
            "new",
            "Trace Case",
            "--path",
            "scenario.txt",
            "--selector",
            "*",
            "--requirement",
            "RFC-0001:C-REQ@0.1.0",
        ]],
    )?;
    assert!(!created.contains("exit: 1"), "{created}");

    let case_path = temp_dir.path().join("gov/conformance/CONF-TRACE-CASE.toml");
    let before = fs::read_to_string(&case_path)?;
    let output = run_commands(
        temp_dir.path(),
        &[
            &[
                "conformance",
                "edit",
                "CONF-TRACE-CASE",
                "requirements[0]",
                "--remove",
            ],
            &[
                "conformance",
                "new",
                "Duplicate Locator",
                "--path",
                "./scenario.txt",
                "--selector",
                "*",
                "--requirement",
                "RFC-0001:C-REQ@0.1.0",
            ],
        ],
    )?;

    assert!(output.contains("exit: 1"), "{output}");
    assert!(
        output.contains("must retain at least one requirement"),
        "{output}"
    );
    assert!(output.contains("same path and selector"), "{output}");
    assert_eq!(fs::read_to_string(&case_path)?, before);
    assert!(
        !temp_dir
            .path()
            .join("gov/conformance/CONF-DUPLICATE-LOCATOR.toml")
            .exists()
    );
    Ok(())
}

#[test]
fn conformance_mutations_ignore_an_unrelated_invalid_case() -> common::TestResult {
    let temp_dir = init_project()?;
    establish_requirement(temp_dir.path())?;
    let setup = run_commands(
        temp_dir.path(),
        &[
            &[
                "conformance",
                "new",
                "Broken Neighbor",
                "--path",
                "scenario.txt",
                "--selector",
                "broken",
                "--requirement",
                "RFC-0001:C-REQ@0.1.0",
            ],
            &[
                "conformance",
                "new",
                "Editable Case",
                "--path",
                "scenario.txt",
                "--selector",
                "edit",
                "--requirement",
                "RFC-0001:C-REQ@0.1.0",
            ],
            &[
                "conformance",
                "new",
                "Deletable Case",
                "--path",
                "scenario.txt",
                "--selector",
                "delete",
                "--requirement",
                "RFC-0001:C-REQ@0.1.0",
            ],
        ],
    )?;
    assert!(!setup.contains("exit: 1"), "{setup}");

    let neighbor = temp_dir
        .path()
        .join("gov/conformance/CONF-BROKEN-NEIGHBOR.toml");
    let invalid = fs::read_to_string(&neighbor)?.replace("0.1.0", "9.9.9");
    fs::write(&neighbor, invalid)?;

    let mutations = run_commands(
        temp_dir.path(),
        &[
            &[
                "conformance",
                "new",
                "Created During Repair",
                "--path",
                "scenario.txt",
                "--selector",
                "create",
                "--requirement",
                "RFC-0001:C-REQ@0.1.0",
            ],
            &[
                "conformance",
                "edit",
                "CONF-EDITABLE-CASE",
                "title",
                "--set",
                "Edited During Repair",
            ],
            &["conformance", "delete", "CONF-DELETABLE-CASE", "--force"],
        ],
    )?;
    assert!(!mutations.contains("exit: 1"), "{mutations}");
    assert!(
        temp_dir
            .path()
            .join("gov/conformance/CONF-CREATED-DURING-REPAIR.toml")
            .exists()
    );
    assert!(
        fs::read_to_string(
            temp_dir
                .path()
                .join("gov/conformance/CONF-EDITABLE-CASE.toml")
        )?
        .contains("Edited During Repair")
    );
    assert!(
        !temp_dir
            .path()
            .join("gov/conformance/CONF-DELETABLE-CASE.toml")
            .exists()
    );

    let checked = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(checked.contains("exit: 1"), "{checked}");
    assert!(checked.contains("E1305"), "{checked}");
    assert!(checked.contains("9.9.9"), "{checked}");
    Ok(())
}

#[test]
fn schema_three_requires_atomic_migration_for_prospective_cases() -> common::TestResult {
    let temp_dir = init_project_at(Some(3))?;
    establish_requirement(temp_dir.path())?;

    let command = run_commands(
        temp_dir.path(),
        &[&[
            "conformance",
            "new",
            "Blocked Case",
            "--path",
            "scenario.txt",
            "--selector",
            "*",
            "--requirement",
            "RFC-0001:C-REQ@0.1.0",
        ]],
    )?;
    assert!(command.contains("E0505"), "{command}");
    assert!(command.contains("govctl migrate"), "{command}");

    fs::create_dir_all(temp_dir.path().join("gov/conformance"))?;
    fs::write(
        temp_dir.path().join("gov/conformance/CONF-EXISTING.toml"),
        r#"[govctl]
id = "CONF-EXISTING"
title = "Existing Case"

[case]
path = "scenario.txt"
selector = "*"
requirements = [{ ref = "RFC-0001:C-REQ", version = "0.1.0" }]
"#,
    )?;
    let migrated = run_commands(temp_dir.path(), &[&["migrate"], &["check"]])?;
    assert!(!migrated.contains("exit: 1"), "{migrated}");
    assert!(migrated.contains("v3 -> v4"), "{migrated}");
    assert!(
        temp_dir
            .path()
            .join("gov/conformance/CONF-EXISTING.toml")
            .exists()
    );
    assert!(fs::read_to_string(temp_dir.path().join("gov/config.toml"))?.contains("version = 4"));
    Ok(())
}

#[test]
fn schema_three_rejects_invalid_prospective_case_without_partial_migration() -> common::TestResult {
    let temp_dir = init_project_at(Some(3))?;
    establish_requirement(temp_dir.path())?;
    fs::write(
        temp_dir.path().join("gov/conformance/CONF-BROKEN.toml"),
        r#"[govctl]
id = "CONF-BROKEN"
title = "Broken Case"

[case]
path = "missing.txt"
selector = "*"
requirements = [{ ref = "RFC-0001:C-REQ", version = "0.1.0" }]
"#,
    )?;

    let normal_load = run_commands(temp_dir.path(), &[&["check"]])?;
    assert!(normal_load.contains("E0505"), "{normal_load}");
    let before = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    let migration = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(migration.contains("E1305"), "{migration}");
    assert!(migration.contains("missing.txt"), "{migration}");
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("gov/config.toml"))?,
        before
    );
    Ok(())
}

#[test]
fn conformance_read_outputs_and_string_list_removal_follow_shared_contracts() -> common::TestResult
{
    let temp_dir = init_project()?;
    establish_requirement(temp_dir.path())?;
    let added_requirement = run_commands(
        temp_dir.path(),
        &[&[
            "clause",
            "new",
            "RFC-0001:C-SECOND",
            "Second trace requirement",
        ]],
    )?;
    assert!(
        !added_requirement.contains("exit: 1"),
        "{added_requirement}"
    );
    let created = run_commands(
        temp_dir.path(),
        &[&[
            "conformance",
            "new",
            "Output Case",
            "--path",
            "scenario.txt",
            "--selector",
            "*",
            "--requirement",
            "RFC-0001:C-REQ@0.1.0",
            "--guard",
            "GUARD-TRACE-GUARD",
        ]],
    )?;
    assert!(!created.contains("exit: 1"), "{created}");

    let list_yaml = run_commands(
        temp_dir.path(),
        &[&["conformance", "list", "--output", "yaml"]],
    )?;
    assert!(list_yaml.contains("id: CONF-OUTPUT-CASE"), "{list_yaml}");

    for format in ["json", "yaml", "toml", "table"] {
        let output = run_commands(
            temp_dir.path(),
            &[&["conformance", "get", "CONF-OUTPUT-CASE", "--output", format]],
        )?;
        assert!(!output.contains("exit: 1"), "{output}");
    }
    for format in ["plain", "json", "yaml"] {
        let output = run_commands(
            temp_dir.path(),
            &[&[
                "conformance",
                "get",
                "CONF-OUTPUT-CASE",
                "title",
                "--output",
                format,
            ]],
        )?;
        assert!(output.contains("Output Case"), "{output}");
        assert!(!output.contains("exit: 1"), "{output}");
    }
    let show = run_commands(
        temp_dir.path(),
        &[&["conformance", "show", "CONF-OUTPUT-CASE"]],
    )?;
    assert!(show.contains("Requirements"), "{show}");
    assert!(!show.contains("[govctl]"), "{show}");

    let invalid = run_commands(
        temp_dir.path(),
        &[
            &[
                "conformance",
                "get",
                "CONF-OUTPUT-CASE",
                "--output",
                "plain",
            ],
            &[
                "conformance",
                "get",
                "CONF-OUTPUT-CASE",
                "title",
                "--output",
                "table",
            ],
        ],
    )?;
    assert_eq!(invalid.matches("error[E0820]").count(), 2, "{invalid}");

    let edits = run_commands(
        temp_dir.path(),
        &[
            &["tag", "new", "case-tag"],
            &[
                "conformance",
                "edit",
                "CONF-OUTPUT-CASE",
                "requirements",
                "--add",
                "RFC-0001:C-SECOND@0.1.0",
            ],
            &["conformance", "get", "CONF-OUTPUT-CASE", "requirements"],
            &[
                "conformance",
                "edit",
                "CONF-OUTPUT-CASE",
                "requirements",
                "--remove",
                "RFC-0001:C-SECOND@0.1.0",
            ],
            &[
                "conformance",
                "edit",
                "CONF-OUTPUT-CASE",
                "requirements[0].version",
                "--set",
                "0.1.0",
            ],
            &[
                "conformance",
                "edit",
                "CONF-OUTPUT-CASE",
                "tags",
                "--add",
                "case-tag",
            ],
            &[
                "conformance",
                "edit",
                "CONF-OUTPUT-CASE",
                "guards",
                "--remove",
                "TRACE",
                "--regex",
            ],
            &[
                "conformance",
                "edit",
                "CONF-OUTPUT-CASE",
                "tags",
                "--remove",
                "--all",
            ],
            &["conformance", "get", "CONF-OUTPUT-CASE", "guards"],
            &["conformance", "get", "CONF-OUTPUT-CASE", "tags"],
        ],
    )?;
    assert!(!edits.contains("exit: 1"), "{edits}");
    assert!(edits.contains("RFC-0001:C-SECOND@0.1.0"), "{edits}");
    assert!(
        !fs::read_to_string(
            temp_dir
                .path()
                .join("gov/conformance/CONF-OUTPUT-CASE.toml")
        )?
        .contains("GUARD-TRACE-GUARD")
    );
    assert!(
        !fs::read_to_string(
            temp_dir
                .path()
                .join("gov/conformance/CONF-OUTPUT-CASE.toml")
        )?
        .contains("C-SECOND")
    );
    Ok(())
}

#[test]
fn schema_three_prospective_cases_gate_all_commands_and_use_bundled_preflight() -> common::TestResult
{
    let temp_dir = init_project_at(Some(3))?;
    establish_requirement(temp_dir.path())?;
    fs::create_dir_all(temp_dir.path().join("gov/conformance"))?;
    fs::write(
        temp_dir.path().join("gov/conformance/CONF-EXISTING.toml"),
        r#"[govctl]
id = "CONF-EXISTING"
title = "Existing Case"
unexpected = "schema-v4 rejects this"

[case]
path = "scenario.txt"
selector = "*"
requirements = [{ ref = "RFC-0001:C-REQ", version = "0.1.0" }]
"#,
    )?;
    fs::write(
        temp_dir.path().join("gov/schema/conformance.schema.json"),
        "{}\n",
    )?;
    let _ = fs::remove_dir_all(temp_dir.path().join(".govctl"));

    let gated = run_commands(
        temp_dir.path(),
        &[&["search", "Existing"], &["tag", "list"], &["status"]],
    )?;
    assert_eq!(gated.matches("error[E0505]").count(), 3, "{gated}");
    assert_eq!(gated.matches("govctl migrate").count(), 3, "{gated}");
    assert!(!temp_dir.path().join(".govctl").exists());

    let before = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    let migration = run_commands(temp_dir.path(), &[&["--dry-run", "migrate"]])?;
    assert!(migration.contains("error[E1301]"), "{migration}");
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("gov/config.toml"))?,
        before
    );
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("gov/schema/conformance.schema.json"))?,
        "{}\n"
    );
    Ok(())
}

#[test]
fn failed_case_graph_preflight_does_not_create_a_missing_schema_directory() -> common::TestResult {
    let temp_dir = init_project_at(Some(3))?;
    establish_requirement(temp_dir.path())?;
    fs::remove_dir_all(temp_dir.path().join("gov/schema"))?;
    fs::create_dir_all(temp_dir.path().join("gov/conformance"))?;
    fs::write(
        temp_dir.path().join("gov/conformance/CONF-BROKEN.toml"),
        r#"[govctl]
id = "CONF-BROKEN"
title = "Broken Case"

[case]
path = "missing.txt"
selector = "*"
requirements = [{ ref = "RFC-0001:C-REQ", version = "0.1.0" }]
"#,
    )?;

    let migration = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(migration.contains("error[E1305]"), "{migration}");
    assert!(!temp_dir.path().join("gov/schema").exists());
    Ok(())
}

#[test]
fn trace_applicability_tracks_rfc_and_clause_lifecycle_without_blocking_transitions()
-> common::TestResult {
    let temp_dir = init_project()?;
    fs::write(temp_dir.path().join("scenario.txt"), "scenario\n")?;
    let setup = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Lifecycle RFC", "--id", "RFC-0001"],
            &["clause", "new", "RFC-0001:C-REQ", "Lifecycle requirement"],
            &[
                "conformance",
                "new",
                "Lifecycle Case",
                "--path",
                "scenario.txt",
                "--selector",
                "*",
                "--requirement",
                "RFC-0001:C-REQ@0.1.0",
            ],
            &[
                "conformance",
                "trace",
                "CONF-LIFECYCLE-CASE",
                "--output",
                "json",
            ],
        ],
    )?;
    assert!(
        setup.contains("\"requirement_applicability\": \"provisional\""),
        "{setup}"
    );

    let candidate = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "finalize", "RFC-0001", "normative"],
            &[
                "conformance",
                "trace",
                "CONF-LIFECYCLE-CASE",
                "--output",
                "json",
            ],
        ],
    )?;
    assert!(
        candidate.contains("\"requirement_applicability\": \"candidate\""),
        "{candidate}"
    );

    let current = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "advance", "RFC-0001", "impl"],
            &[
                "conformance",
                "trace",
                "CONF-LIFECYCLE-CASE",
                "--output",
                "json",
            ],
        ],
    )?;
    assert!(
        current.contains("\"requirement_applicability\": \"current\""),
        "{current}"
    );

    let stale = run_commands(
        temp_dir.path(),
        &[
            &["clause", "deprecate", "RFC-0001:C-REQ", "--force"],
            &[
                "conformance",
                "trace",
                "CONF-LIFECYCLE-CASE",
                "--output",
                "json",
            ],
        ],
    )?;
    assert!(!stale.contains("exit: 1"), "{stale}");
    assert!(
        stale.contains("\"requirement_applicability\": \"stale\""),
        "{stale}"
    );
    Ok(())
}

#[test]
fn clause_and_guard_deletion_report_conformance_case_referrers() -> common::TestResult {
    let temp_dir = init_project()?;
    establish_requirement(temp_dir.path())?;
    let setup = run_commands(
        temp_dir.path(),
        &[&[
            "conformance",
            "new",
            "Deletion Case",
            "--path",
            "scenario.txt",
            "--selector",
            "*",
            "--requirement",
            "RFC-0001:C-REQ@0.1.0",
            "--guard",
            "GUARD-TRACE-GUARD",
        ]],
    )?;
    assert!(!setup.contains("exit: 1"), "{setup}");

    let blocked = run_commands(
        temp_dir.path(),
        &[
            &["clause", "delete", "RFC-0001:C-REQ", "--force"],
            &["guard", "delete", "GUARD-TRACE-GUARD", "--force"],
        ],
    )?;
    assert_eq!(blocked.matches("exit: 1").count(), 2, "{blocked}");
    assert_eq!(
        blocked.matches("CONF-DELETION-CASE").count(),
        2,
        "{blocked}"
    );
    assert!(
        temp_dir
            .path()
            .join("gov/rfc/RFC-0001/clauses/C-REQ.toml")
            .exists()
    );
    assert!(temp_dir.path().join("gov/guard/trace-guard.toml").exists());
    Ok(())
}
