mod common;

use common::{first_work_id, init_project, run_commands};
use std::fs;

fn assert_success_with(output: &str, marker: &str) {
    assert!(output.contains(marker), "output: {output}");
    assert!(output.contains("exit: 0"), "output: {output}");
}

fn successful_payload(output: &str) -> &str {
    output
        .split_once('\n')
        .and_then(|(_, body)| body.strip_suffix("exit: 0\n\n"))
        .expect("single successful command output")
        .trim_end()
}

fn get_value(resource: &str, output: &str) -> serde_json::Value {
    let payload = successful_payload(output);
    if matches!(resource, "rfc" | "clause") {
        serde_json::from_str(payload).expect("complete get JSON")
    } else {
        serde_json::to_value(toml::from_str::<toml::Value>(payload).expect("complete get TOML"))
            .expect("TOML converts to JSON value")
    }
}

#[test]
fn test_show_structured_formats_are_complete_for_every_resource() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Show RFC"],
            &["clause", "new", "RFC-0001:C-SHOW", "Show Clause"],
            &["adr", "new", "Show ADR"],
            &["work", "new", "Show Work"],
            &["guard", "new", "Show Guard"],
        ],
    )?;
    let work_id = first_work_id(&common::today());
    let resources = [
        ("rfc", "RFC-0001", "rfc_id", "sections", "[[sections]]"),
        ("clause", "RFC-0001:C-SHOW", "clause_id", "text", "text ="),
        ("adr", "ADR-0001", "govctl", "content", "[content]"),
        ("work", work_id.as_str(), "govctl", "content", "[content]"),
        ("guard", "GUARD-SHOW-GUARD", "govctl", "check", "[check]"),
    ];

    for (resource, id, identity_key, content_key, toml_content_marker) in resources {
        let get = run_commands(temp_dir.path(), &[&[resource, "get", id]])?;
        let expected = get_value(resource, &get);

        let json = run_commands(
            temp_dir.path(),
            &[&[resource, "show", id, "--output", "json"]],
        )?;
        assert_success_with(&json, &format!("\"{identity_key}\""));
        assert!(json.contains(&format!("\"{content_key}\"")), "{json}");
        let json_value: serde_json::Value =
            serde_json::from_str(successful_payload(&json)).expect("show JSON");
        assert_eq!(json_value, expected, "resource: {resource}");

        let yaml = run_commands(
            temp_dir.path(),
            &[&[resource, "show", id, "--output", "yaml"]],
        )?;
        assert_success_with(&yaml, &format!("{identity_key}:"));
        assert!(yaml.contains(&format!("{content_key}:")), "{yaml}");
        let yaml_value: serde_json::Value =
            serde_yaml::from_str(successful_payload(&yaml)).expect("show YAML");
        assert_eq!(yaml_value, expected, "resource: {resource}");

        let toml = run_commands(
            temp_dir.path(),
            &[&[resource, "show", id, "--output", "toml"]],
        )?;
        let toml_identity_marker = if identity_key == "govctl" {
            "[govctl]".to_string()
        } else {
            format!("{identity_key} =")
        };
        assert_success_with(&toml, &toml_identity_marker);
        assert!(toml.contains(toml_content_marker), "{toml}");
        let toml_value = serde_json::to_value(
            toml::from_str::<toml::Value>(successful_payload(&toml)).expect("show TOML"),
        )
        .expect("TOML converts to JSON value");
        assert_eq!(toml_value, expected, "resource: {resource}");

        for format in ["json", "yaml", "toml"] {
            let conflict = run_commands(
                temp_dir.path(),
                &[&[resource, "show", id, "--history", "--output", format]],
            )?;
            assert!(conflict.contains("error[E0802]"), "output: {conflict}");
            assert!(conflict.contains("exit: 1"), "output: {conflict}");
        }
    }
    Ok(())
}

#[test]
fn test_rfc_show_filters_obsolete_content_but_render_keeps_history() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Projection RFC"],
            &["clause", "new", "RFC-0001:C-ACTIVE", "Active Clause"],
            &[
                "clause",
                "edit",
                "RFC-0001:C-ACTIVE",
                "text",
                "--set",
                "ACTIVE BODY",
            ],
            &["clause", "new", "RFC-0001:C-OLD", "Old Clause"],
            &[
                "clause",
                "edit",
                "RFC-0001:C-OLD",
                "text",
                "--set",
                "OBSOLETE BODY",
            ],
            &["clause", "new", "RFC-0001:C-NEW", "Replacement Clause"],
            &[
                "clause",
                "supersede",
                "RFC-0001:C-OLD",
                "--by",
                "RFC-0001:C-NEW",
                "--force",
            ],
            &["tag", "new", "projection"],
            &["rfc", "add", "RFC-0001", "tags", "projection"],
        ],
    )?;

    let current = run_commands(temp_dir.path(), &[&["rfc", "show", "RFC-0001"]])?;
    assert!(current.contains("ACTIVE BODY"), "output: {current}");
    assert!(
        current.contains("**Status:** superseded"),
        "output: {current}"
    );
    assert!(!current.contains("OBSOLETE BODY"), "output: {current}");

    let history = run_commands(
        temp_dir.path(),
        &[&["rfc", "show", "RFC-0001", "--history"]],
    )?;
    assert!(history.contains("ACTIVE BODY"), "output: {history}");
    assert!(history.contains("OBSOLETE BODY"), "output: {history}");

    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "new", "Replacement RFC"],
            &[
                "rfc",
                "supersede",
                "RFC-0001",
                "--by",
                "RFC-0002",
                "--force",
            ],
        ],
    )?;
    let deprecated = run_commands(temp_dir.path(), &[&["rfc", "show", "RFC-0001"]])?;
    assert!(deprecated.contains("**Status:** deprecated"));
    assert!(deprecated.contains("**Owners:** @test-user"));
    assert!(deprecated.contains("**Tags:** `projection`"));
    assert!(deprecated.contains("**Superseded by:** RFC-0002"));
    assert!(!deprecated.contains("## 1."), "output: {deprecated}");
    assert!(!deprecated.contains("ACTIVE BODY"), "output: {deprecated}");

    let replacement_history = run_commands(
        temp_dir.path(),
        &[&["rfc", "show", "RFC-0002", "--history"]],
    )?;
    assert!(
        replacement_history.contains("**Supersedes:** RFC-0001"),
        "output: {replacement_history}"
    );

    run_commands(temp_dir.path(), &[&["rfc", "render", "RFC-0001"]])?;
    let rendered = fs::read_to_string(temp_dir.path().join("docs/rfc/RFC-0001.md"))?;
    assert!(rendered.contains("**Owners:** @test-user"));
    assert!(rendered.contains("**Tags:** `projection`"));
    assert!(rendered.contains("ACTIVE BODY"), "rendered: {rendered}");
    assert!(rendered.contains("OBSOLETE BODY"), "rendered: {rendered}");
    Ok(())
}

#[test]
fn test_content_equivalent_resources_accept_explicit_human_projection_matrix() -> common::TestResult
{
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["work", "new", "Projection Work"],
            &["guard", "new", "Projection Guard"],
        ],
    )?;
    let work_id = first_work_id(&common::today());

    for format in ["table", "plain"] {
        let work_current = run_commands(
            temp_dir.path(),
            &[&["work", "show", work_id.as_str(), "--output", format]],
        )?;
        let work_history = run_commands(
            temp_dir.path(),
            &[&[
                "work",
                "show",
                work_id.as_str(),
                "--output",
                format,
                "--history",
            ]],
        )?;
        assert_success_with(&work_current, "Projection Work");
        assert_eq!(
            successful_payload(&work_current),
            successful_payload(&work_history),
            "work format: {format}"
        );

        let guard_current = run_commands(
            temp_dir.path(),
            &[&[
                "guard",
                "show",
                "GUARD-PROJECTION-GUARD",
                "--output",
                format,
            ]],
        )?;
        let guard_history = run_commands(
            temp_dir.path(),
            &[&[
                "guard",
                "show",
                "GUARD-PROJECTION-GUARD",
                "--output",
                format,
                "--history",
            ]],
        )?;
        assert_success_with(&guard_current, "Projection Guard");
        assert_eq!(
            successful_payload(&guard_current),
            successful_payload(&guard_history),
            "guard format: {format}"
        );
    }
    Ok(())
}

#[test]
fn test_superseded_adr_show_requires_history_for_body_content() -> common::TestResult {
    let temp_dir = init_project()?;
    run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Old Decision"],
            &["adr", "set", "ADR-0001", "context", "OBSOLETE ADR CONTEXT"],
            &["adr", "set", "ADR-0001", "decision", "Old decision"],
            &["adr", "set", "ADR-0001", "consequences", "Old consequences"],
            &["adr", "accept", "ADR-0001", "--force"],
            &["adr", "new", "New Decision"],
            &["adr", "set", "ADR-0002", "context", "New context"],
            &["adr", "set", "ADR-0002", "decision", "New decision"],
            &["adr", "set", "ADR-0002", "consequences", "New consequences"],
            &["adr", "accept", "ADR-0002", "--force"],
            &[
                "adr",
                "supersede",
                "ADR-0001",
                "--by",
                "ADR-0002",
                "--force",
            ],
        ],
    )?;

    let current = run_commands(temp_dir.path(), &[&["adr", "show", "ADR-0001"]])?;
    assert!(
        current.contains("**Status:** superseded"),
        "output: {current}"
    );
    assert!(current.contains("**Superseded by:** ADR-0002"));
    assert!(!current.contains("OBSOLETE ADR CONTEXT"));

    let history = run_commands(
        temp_dir.path(),
        &[&["adr", "show", "ADR-0001", "--history"]],
    )?;
    assert!(
        history.contains("OBSOLETE ADR CONTEXT"),
        "output: {history}"
    );
    Ok(())
}
