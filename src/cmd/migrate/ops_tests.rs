use super::*;
use crate::config::Config;
use std::fs;

fn test_config(temp_dir: &tempfile::TempDir) -> Config {
    let mut config = Config {
        gov_root: temp_dir.path().join("gov"),
        ..Config::default()
    };
    config.paths.docs_output = temp_dir.path().join("docs");
    config
}

#[test]
fn execute_ops_removes_created_files_when_later_apply_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = test_config(&temp_dir);
    fs::create_dir_all(&config.gov_root)?;
    let created = config.gov_root.join("created.txt");
    let bad_target = config.gov_root.join("bad-target");
    fs::create_dir_all(&bad_target)?;

    let result = execute_ops(
        &config,
        &[
            FileOp::Write {
                path: created.clone(),
                content: "created".to_string(),
            },
            FileOp::Write {
                path: bad_target,
                content: "cannot replace directory".to_string(),
            },
        ],
    );

    assert!(result.is_err());
    assert!(
        !created.exists(),
        "created migration target should be removed on rollback"
    );
    assert!(!config.gov_root.join(".migrate-stage").exists());
    assert!(!config.gov_root.join(".migrate-backup").exists());
    Ok(())
}

#[test]
fn execute_ops_restores_modified_and_deleted_files_when_later_apply_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = test_config(&temp_dir);
    fs::create_dir_all(&config.gov_root)?;
    let modified = config.gov_root.join("modified.txt");
    let deleted = config.gov_root.join("deleted.txt");
    let bad_target = config.gov_root.join("bad-target");
    fs::write(&modified, "old")?;
    fs::write(&deleted, "gone")?;
    fs::create_dir_all(&bad_target)?;

    let result = execute_ops(
        &config,
        &[
            FileOp::Write {
                path: modified.clone(),
                content: "new".to_string(),
            },
            FileOp::Delete {
                path: deleted.clone(),
            },
            FileOp::Write {
                path: bad_target,
                content: "cannot replace directory".to_string(),
            },
        ],
    );

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(modified)?, "old");
    assert_eq!(fs::read_to_string(deleted)?, "gone");
    assert!(!config.gov_root.join(".migrate-stage").exists());
    assert!(!config.gov_root.join(".migrate-backup").exists());
    Ok(())
}

#[test]
fn execute_ops_retains_backup_and_reports_e0903_when_rollback_fails()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config = test_config(&temp_dir);
    fs::create_dir_all(&config.gov_root)?;
    let victim = config.gov_root.join("victim");
    let bad_target = config.gov_root.join("bad-target");
    fs::write(&victim, "old")?;
    fs::create_dir(&bad_target)?;

    let error = match execute_ops(
        &config,
        &[
            FileOp::Write {
                path: victim.clone(),
                content: "new".to_string(),
            },
            FileOp::Delete {
                path: victim.clone(),
            },
            FileOp::Write {
                path: victim.join("child"),
                content: "forces victim to become a directory".to_string(),
            },
            FileOp::Write {
                path: bad_target,
                content: "cannot replace directory".to_string(),
            },
        ],
    ) {
        Ok(()) => return Err("rollback unexpectedly succeeded".into()),
        Err(error) => error,
    };

    assert_eq!(error.code, DiagnosticCode::E0903UnexpectedError);
    assert!(error.message.contains("restoration may be incomplete"));
    assert!(error.message.contains("recovery backup retained at"));
    assert!(!config.gov_root.join(".migrate-stage").exists());
    assert!(config.gov_root.join(".migrate-backup").exists());
    Ok(())
}
