//! Delete command tests - clause and work item deletion safeguards.

mod common;

use common::{
    TestResult, init_project, normalize_output, run_commands, run_dynamic_commands, today,
};
use std::fs;

/// Test: Delete clause - safeguard prevents deleting from normative RFC
#[test]
fn test_delete_clause_safeguard_normative() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();

    // Create normative RFC with clause
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"#:schema ../../schema/rfc.schema.json

[govctl]
schema = 1
id = "RFC-0001"
title = "Normative RFC"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Specification"
clauses = ["clauses/C-LOCKED.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-01"
added = ["Initial release"]
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-LOCKED.toml"),
        r#"#:schema ../../schema/clause.schema.json

[govctl]
schema = 1
id = "C-LOCKED"
title = "Locked Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "This clause cannot be deleted - RFC is normative."
"#,
    )?;

    // Try to delete the clause (should fail)
    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "delete", "RFC-0001:C-LOCKED", "-f"]],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete clause - successful deletion from draft RFC
#[test]
fn test_delete_clause_success_draft() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();

    // Create draft RFC with two clauses
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"#:schema ../../schema/rfc.schema.json

[govctl]
schema = 1
id = "RFC-0001"
title = "Draft RFC"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Specification"
clauses = ["clauses/C-KEEP.toml", "clauses/C-DELETE.toml"]

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
notes = "Initial draft"
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-KEEP.toml"),
        r#"#:schema ../../schema/clause.schema.json

[govctl]
schema = 1
id = "C-KEEP"
title = "Keep This Clause"
kind = "normative"
status = "active"
since = "0.1.0"

[content]
text = "This clause will remain."
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-DELETE.toml"),
        r#"#:schema ../../schema/clause.schema.json

[govctl]
schema = 1
id = "C-DELETE"
title = "Delete This Clause"
kind = "normative"
status = "active"
since = "0.1.0"

[content]
text = "This clause will be deleted."
"#,
    )?;

    // Delete the clause with force flag, then verify
    let output = run_commands(
        temp_dir.path(),
        &[
            &["clause", "delete", "RFC-0001:C-DELETE", "-f"],
            &["clause", "list", "RFC-0001"],
            &["check"],
        ],
    )?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

#[test]
fn test_delete_clause_safeguard_referenced_by_artifacts() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let work_id = format!("WI-{date}-001");

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"#:schema ../../schema/rfc.schema.json

[govctl]
schema = 1
id = "RFC-0001"
title = "Draft RFC"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["test@example.com"]
created = "2026-01-01"

[[sections]]
title = "Specification"
clauses = ["clauses/C-DELETE.toml"]

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
notes = "Initial draft"
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-DELETE.toml"),
        r#"#:schema ../../schema/clause.schema.json

[govctl]
schema = 1
id = "C-DELETE"
title = "Referenced Clause"
kind = "normative"
status = "active"
since = "0.1.0"

[content]
text = "This clause is still referenced."
"#,
    )?;

    let commands: Vec<Vec<String>> = vec![
        vec![
            "rfc".to_string(),
            "new".to_string(),
            "Referencing RFC".to_string(),
        ],
        vec![
            "rfc".to_string(),
            "add".to_string(),
            "RFC-0002".to_string(),
            "refs".to_string(),
            "RFC-0001:C-DELETE".to_string(),
        ],
        vec![
            "adr".to_string(),
            "new".to_string(),
            "Referencing ADR".to_string(),
        ],
        vec![
            "adr".to_string(),
            "add".to_string(),
            "ADR-0001".to_string(),
            "refs".to_string(),
            "RFC-0001:C-DELETE".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Referencing work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            work_id.clone(),
            "refs".to_string(),
            "RFC-0001:C-DELETE".to_string(),
        ],
        vec![
            "guard".to_string(),
            "new".to_string(),
            "Clause guard".to_string(),
        ],
        vec![
            "guard".to_string(),
            "add".to_string(),
            "GUARD-CLAUSE-GUARD".to_string(),
            "refs".to_string(),
            "RFC-0001:C-DELETE".to_string(),
        ],
        vec![
            "clause".to_string(),
            "delete".to_string(),
            "RFC-0001:C-DELETE".to_string(),
            "-f".to_string(),
        ],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    assert!(output.contains("exit: 1"), "output: {output}");
    assert!(output.contains("Cannot delete clause"), "output: {output}");
    assert!(output.contains("RFC-0002"), "output: {output}");
    assert!(output.contains("ADR-0001"), "output: {output}");
    assert!(output.contains(&work_id), "output: {output}");
    assert!(output.contains("GUARD-CLAUSE-GUARD"), "output: {output}");
    assert!(
        rfc_dir.join("clauses/C-DELETE.toml").exists(),
        "referenced clause should remain on disk"
    );

    Ok(())
}

/// Test: Delete work item - safeguard prevents deleting active work item
#[test]
fn test_delete_work_safeguard_active() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let wi1 = format!("WI-{}-001", date);

    // Create active work item
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Active work item".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "chore: Test criterion".to_string(),
        ],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete work item - safeguard prevents deleting done work item
#[test]
fn test_delete_work_safeguard_done() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let wi1 = format!("WI-{}-001", date);

    // Create and complete work item
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Completed work item".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "chore: Test criterion".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "Test".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi1.clone(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete work item - successful deletion of queued work item
#[test]
fn test_delete_work_success_queue() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let wi1 = format!("WI-{}-001", date);

    // Create two queued work items
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Keep this work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Delete this work item".to_string(),
        ],
        vec!["work".to_string(), "list".to_string()],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
        vec!["work".to_string(), "list".to_string()],
        vec!["check".to_string()],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands)?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete work item - safeguard prevents deletion when referenced
#[test]
fn test_delete_work_safeguard_referenced() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let wi1 = format!("WI-{}-001", date);
    let wi2 = format!("WI-{}-002", date);

    // Create two work items where wi2 references wi1
    let setup_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Referenced work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Work item with reference".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "refs".to_string(),
            wi1.clone(),
        ],
    ];

    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands)?;

    // Try to delete wi1 (should fail because wi2 references it)
    let delete_commands: Vec<Vec<String>> = vec![vec![
        "work".to_string(),
        "delete".to_string(),
        wi1.clone(),
        "-f".to_string(),
    ]];

    let output = run_dynamic_commands(temp_dir.path(), &delete_commands)?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);

    Ok(())
}

/// Test: Delete work item - safeguard prevents deletion when depended on
#[test]
fn test_delete_work_safeguard_depended_on() -> TestResult {
    let temp_dir = init_project()?;
    let date = today();
    let wi1 = format!("WI-{}-001", date);
    let wi2 = format!("WI-{}-002", date);

    let setup_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Dependency work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Dependent work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "depends_on".to_string(),
            wi1.clone(),
        ],
    ];

    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands)?;

    let output = run_dynamic_commands(
        temp_dir.path(),
        &[vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ]],
    )?;

    assert!(output.contains("exit: 1"), "output: {}", output);
    assert!(
        output.contains("Cannot delete work item"),
        "output: {}",
        output
    );
    assert!(output.contains(&wi2), "output: {}", output);
    Ok(())
}
