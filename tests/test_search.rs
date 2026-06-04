mod common;

use common::{init_project, run_commands, write_guard_with_timeout};
use rusqlite::Connection;
use std::fs;
use std::path::Path;

fn write_clause(
    dir: &Path,
    rfc_id: &str,
    clause_id: &str,
    tag: Option<&str>,
) -> common::TestResult {
    let tags = tag
        .map(|tag| format!("tags = [\"{tag}\"]\n"))
        .unwrap_or_default();
    let path = dir
        .join("gov/rfc")
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_id}.toml"));
    fs::write(
        path,
        format!(
            r#"[govctl]
schema = 1
id = "{clause_id}"
title = "Cache Clause"
kind = "normative"
status = "active"
anchors = ["cache-anchor"]
since = "0.1.0"
superseded_by = "C-NEXT"
{tags}
[content]
text = "cache clause body"
"#
        ),
    )?;
    Ok(())
}

fn write_search_rfc(dir: &Path) -> common::TestResult {
    let rfc_dir = dir.join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;
    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"[govctl]
schema = 1
id = "RFC-0001"
title = "Cache Policy"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@test-user"]
created = "2026-01-01"
refs = ["ADR-0001"]
tags = ["rfctag"]

[[sections]]
title = "Cache Specification"
clauses = ["clauses/C-CACHE.toml"]

[[changelog]]
version = "0.1.0"
date = "2026-01-01"
notes = "cache changelog note"
added = ["cache added"]
changed = ["cache changed"]
deprecated = ["cache deprecated"]
removed = ["cache removed"]
fixed = ["cache fixed"]
security = ["cache security"]
"#,
    )?;
    Ok(())
}

fn write_adr(dir: &Path) -> common::TestResult {
    fs::write(
        dir.join("gov/adr/ADR-0001-cache-decision.toml"),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Cache Decision"
status = "superseded"
date = "2026-01-01"
superseded_by = "ADR-0002"
refs = ["RFC-0001"]
tags = ["adrtag"]

[content]
context = "cache context"
decision = "Use cache search"
consequences = "Searchable ADR"

[[content.alternatives]]
text = "cache alternative"
status = "rejected"
pros = ["cache pro"]
cons = ["cache con"]
rejection_reason = "cache reason"
"#,
    )?;
    Ok(())
}

fn write_work(dir: &Path, id: &str, title: &str, description: &str) -> common::TestResult {
    fs::write(
        dir.join("gov/work/2026-01-01-search-work.toml"),
        format!(
            r#"[govctl]
schema = 1
id = "{id}"
title = "{title}"
status = "active"
created = "2026-01-01"
started = "2026-01-01"
refs = ["RFC-0001"]
depends_on = ["WI-2026-01-01-999"]
tags = ["worktag"]

[content]
description = "{description}"
notes = ["cache durable note"]

[[content.acceptance_criteria]]
text = "cache criterion"
status = "pending"
category = "chore"
"#
        ),
    )?;
    Ok(())
}

fn write_work_with_journal(dir: &Path) -> common::TestResult {
    fs::write(
        dir.join("gov/work/2026-01-01-journal-work.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-002"
title = "Journal Work"
status = "active"
created = "2026-01-01"
started = "2026-01-01"

[content]
description = "stable searchable body"

[[content.journal]]
date = "2026-01-01"
content = "journalonlytoken"
"#,
    )?;
    Ok(())
}

#[test]
fn test_search_finds_all_artifact_types_and_output_formats() -> common::TestResult {
    let temp_dir = init_project()?;
    write_search_rfc(temp_dir.path())?;
    write_clause(temp_dir.path(), "RFC-0001", "C-CACHE", Some("searchtag"))?;
    write_adr(temp_dir.path())?;
    write_work(
        temp_dir.path(),
        "WI-2026-01-01-001",
        "Cache Work",
        "cache work body",
    )?;
    write_guard_with_timeout(
        temp_dir.path(),
        "GUARD-CACHE",
        "echo cache",
        Some("cache"),
        5,
    )?;

    let json = run_commands(
        temp_dir.path(),
        &[&["search", "cache", "-o", "json", "-n", "10"]],
    )?;
    assert!(json.contains("\"kind\": \"rfc\""), "{json}");
    assert!(json.contains("\"kind\": \"clause\""), "{json}");
    assert!(json.contains("\"kind\": \"adr\""), "{json}");
    assert!(json.contains("\"kind\": \"work\""), "{json}");
    assert!(json.contains("\"kind\": \"guard\""), "{json}");
    assert!(json.contains("\"path\""), "{json}");
    assert!(json.contains("\"snippet\""), "{json}");

    let plain = run_commands(
        temp_dir.path(),
        &[&["search", "cache", "--type", "guard", "-o", "plain"]],
    )?;
    assert!(plain.contains("GUARD-CACHE"), "{plain}");
    assert!(!plain.contains("ADR-0001"), "{plain}");

    let tagged = run_commands(
        temp_dir.path(),
        &[&["search", "cache", "--tag", "searchtag", "-o", "plain"]],
    )?;
    assert!(tagged.contains("RFC-0001:C-CACHE"), "{tagged}");
    assert!(!tagged.contains("ADR-0001"), "{tagged}");

    let table = run_commands(temp_dir.path(), &[&["search", "cache", "--type", "rfc"]])?;
    assert!(table.contains("Kind"), "{table}");
    assert!(table.contains("ID"), "{table}");
    assert!(table.contains("Title"), "{table}");
    assert!(table.contains("Path"), "{table}");
    assert!(table.contains("Snippet"), "{table}");

    let stemmed = run_commands(
        temp_dir.path(),
        &[&["search", "caching", "--type", "work", "-o", "plain"]],
    )?;
    assert!(stemmed.contains("WI-2026-01-01-001"), "{stemmed}");

    let clause = run_commands(
        temp_dir.path(),
        &[&["search", "cache", "--type", "clause", "-o", "plain"]],
    )?;
    assert!(clause.contains("RFC-0001:C-CACHE"), "{clause}");

    let adr = run_commands(
        temp_dir.path(),
        &[&[
            "search",
            "cache",
            "--type",
            "adr",
            "--reindex",
            "-o",
            "plain",
        ]],
    )?;
    assert!(adr.contains("ADR-0001"), "{adr}");
    Ok(())
}

#[test]
fn test_search_sync_updates_changed_and_deleted_artifacts() -> common::TestResult {
    let temp_dir = init_project()?;
    let work_id = "WI-2026-01-01-001";
    write_work(temp_dir.path(), work_id, "Needle Work", "needle body")?;

    let initial = run_commands(
        temp_dir.path(),
        &[&["search", "needle", "--type", "work", "-o", "plain"]],
    )?;
    assert!(initial.contains(work_id), "{initial}");

    write_work(temp_dir.path(), work_id, "Haystack Work", "haystack body")?;
    let stale_query = run_commands(
        temp_dir.path(),
        &[&["search", "needle", "--type", "work", "-o", "plain"]],
    )?;
    assert!(!stale_query.contains(work_id), "{stale_query}");

    let changed_query = run_commands(
        temp_dir.path(),
        &[&["search", "haystack", "--type", "work", "-o", "plain"]],
    )?;
    assert!(changed_query.contains(work_id), "{changed_query}");

    fs::remove_file(temp_dir.path().join("gov/work/2026-01-01-search-work.toml"))?;
    let deleted_query = run_commands(
        temp_dir.path(),
        &[&["search", "haystack", "--type", "work", "-o", "plain"]],
    )?;
    assert!(!deleted_query.contains(work_id), "{deleted_query}");
    Ok(())
}

#[test]
fn test_search_repairs_manifest_fts_divergence() -> common::TestResult {
    let temp_dir = init_project()?;
    let work_id = "WI-2026-01-01-001";
    let work_path = temp_dir.path().join("gov/work/2026-01-01-search-work.toml");
    let db_path = temp_dir.path().join(".govctl/index.db");
    write_work(temp_dir.path(), work_id, "Needle Work", "needle body")?;

    let initial = run_commands(
        temp_dir.path(),
        &[&["search", "needle", "--type", "work", "-o", "plain"]],
    )?;
    assert!(initial.contains(work_id), "{initial}");

    {
        let connection = Connection::open(&db_path)?;
        connection.execute(
            "DELETE FROM search_fts WHERE kind = 'work' AND id = ?1",
            [work_id],
        )?;
    }
    let repaired = run_commands(
        temp_dir.path(),
        &[&["search", "needle", "--type", "work", "-o", "plain"]],
    )?;
    assert!(repaired.contains(work_id), "{repaired}");

    {
        let connection = Connection::open(&db_path)?;
        connection.execute(
            "DELETE FROM search_manifest WHERE kind = 'work' AND id = ?1",
            [work_id],
        )?;
    }
    fs::remove_file(work_path)?;
    let orphan_removed = run_commands(
        temp_dir.path(),
        &[&["search", "needle", "--type", "work", "-o", "plain"]],
    )?;
    assert!(!orphan_removed.contains(work_id), "{orphan_removed}");
    Ok(())
}

#[test]
fn test_search_does_not_index_legacy_work_journal() -> common::TestResult {
    let temp_dir = init_project()?;
    write_work_with_journal(temp_dir.path())?;

    let rendered = run_commands(temp_dir.path(), &[&["work", "show", "WI-2026-01-01-002"]])?;
    assert!(rendered.contains("journalonlytoken"), "{rendered}");

    let search = run_commands(
        temp_dir.path(),
        &[&[
            "search",
            "journalonlytoken",
            "--type",
            "work",
            "-o",
            "plain",
        ]],
    )?;
    assert!(!search.contains("WI-2026-01-01-002"), "{search}");
    Ok(())
}

#[test]
fn test_search_rebuilds_unversioned_existing_fts_table() -> common::TestResult {
    let temp_dir = init_project()?;
    let db_dir = temp_dir.path().join(".govctl");
    fs::create_dir_all(&db_dir)?;
    {
        let connection = Connection::open(db_dir.join("index.db"))?;
        connection.execute_batch(
            "
            CREATE TABLE search_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE VIRTUAL TABLE search_fts USING fts5(
                kind UNINDEXED,
                id UNINDEXED,
                title,
                path UNINDEXED,
                tags UNINDEXED,
                status UNINDEXED,
                body
            );
            ",
        )?;
    }
    write_work(
        temp_dir.path(),
        "WI-2026-01-01-003",
        "Cache Work",
        "cache body",
    )?;

    let search = run_commands(
        temp_dir.path(),
        &[&["search", "caching", "--type", "work", "-o", "plain"]],
    )?;
    assert!(search.contains("WI-2026-01-01-003"), "{search}");
    Ok(())
}
