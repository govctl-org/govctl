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
    Ok(())
}
