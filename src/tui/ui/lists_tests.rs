use super::super::super::app::View;
use super::super::test_support::{adr, project_index, render_app, rfc, work_item};
use super::*;
use crate::cmd::search::SearchResult;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::loop_state::LoopState;
use crate::model::{
    AdrStatus, GuardCheck, GuardEntry, GuardMeta, GuardSpec, Release, RfcPhase, RfcStatus,
    WorkItemStatus,
};
use crate::tui::data::{TuiClauseEntry, TuiLoopEntry, TuiTagSummary};
use std::collections::BTreeMap;

#[test]
fn list_renderers_draw_table_rows() -> Result<(), Box<dyn std::error::Error>> {
    let rendered = render_list(View::RfcList, draw_rfc)?;
    assert!(rendered.iter().any(|line| line.contains("RFC-0001")));
    assert!(rendered.iter().any(|line| line.contains("RFC title")));
    assert!(rendered.iter().any(|line| line.contains("normative")));

    let rendered = render_list(View::AdrList, draw_adr)?;
    assert!(rendered.iter().any(|line| line.contains("ADR-0001")));
    assert!(rendered.iter().any(|line| line.contains("ADR title")));
    assert!(rendered.iter().any(|line| line.contains("accepted")));

    let rendered = render_list(View::WorkList, draw_work)?;
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("WI-2026-01-01-001"))
    );
    assert!(rendered.iter().any(|line| line.contains("Work title")));
    assert!(rendered.iter().any(|line| line.contains("active")));
    Ok(())
}

#[test]
fn cockpit_list_renderers_draw_search_loop_and_diagnostic_rows()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(list_project_index());
    app.view = View::Search;
    app.search_results.push(SearchResult {
        kind: "rfc".to_string(),
        id: "RFC-0001".to_string(),
        title: "RFC title".to_string(),
        path: "gov/rfc/RFC-0001/rfc.toml".to_string(),
        snippet: "human cockpit".to_string(),
        score: None,
        status: Some("normative".to_string()),
    });
    let rendered = render_list_app(app, draw_search)?;
    assert!(rendered.iter().any(|line| line.contains("human cockpit")));

    let mut app = App::new(list_project_index());
    app.view = View::LoopList;
    app.supplement.loops.push(TuiLoopEntry {
        id: "LOOP-2026-06-06-001".to_string(),
        state: Some(loop_state()?),
        diagnostic: None,
    });
    let rendered = render_list_app(app, draw_loop)?;
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("LOOP-2026-06-06-001"))
    );
    assert!(rendered.iter().any(|line| line.contains("continue")));

    let mut app = App::new(list_project_index());
    app.view = View::DiagnosticList;
    app.supplement.diagnostics.push(Diagnostic::new(
        DiagnosticCode::E0901IoError,
        "diagnostic for humans",
        "gov/work/WI-2026-01-01-001.toml",
    ));
    let rendered = render_list_app(app, draw_diagnostics)?;
    assert!(rendered.iter().any(|line| line.contains("E0901")));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("diagnostic for humans"))
    );

    let mut app = App::new(list_project_index());
    app.view = View::Search;
    app.search_error = Some(Diagnostic::new(
        DiagnosticCode::E0806InvalidPattern,
        "invalid search syntax",
        "search",
    ));
    let rendered = render_list_app(app, draw_search)?;
    assert!(rendered.iter().any(|line| line.contains("Search failed")));
    assert!(rendered.iter().any(|line| line.contains("E0806")));
    Ok(())
}

#[test]
fn supplemental_list_renderers_draw_clause_guard_release_and_tag_rows()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(list_project_index());
    app.view = View::ClauseList;
    app.supplement.clauses.push(TuiClauseEntry {
        rfc_id: "RFC-0001".to_string(),
        clause: super::super::test_support::clause("C-LIST", "Clause row", "Clause body"),
    });
    let rendered = render_list_app(app, draw_clause)?;
    assert!(rendered.iter().any(|line| line.contains("C-LIST")));
    assert!(rendered.iter().any(|line| line.contains("Clause row")));

    let mut app = App::new(list_project_index());
    app.view = View::GuardList;
    app.supplement.guards.push(guard_entry());
    let rendered = render_list_app(app, draw_guard)?;
    assert!(rendered.iter().any(|line| line.contains("GUARD-LIST")));
    assert!(rendered.iter().any(|line| line.contains("cargo test")));

    let mut app = App::new(list_project_index());
    app.view = View::ReleaseList;
    app.supplement.releases.push(Release {
        version: "0.9.2".to_string(),
        date: "2026-06-05".to_string(),
        refs: vec!["WI-2026-01-01-001".to_string()],
    });
    let rendered = render_list_app(app, draw_release)?;
    assert!(rendered.iter().any(|line| line.contains("0.9.2")));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("WI-2026-01-01-001"))
    );

    let mut app = App::new(list_project_index());
    app.view = View::TagList;
    app.supplement.tags.push(TuiTagSummary {
        name: "release".to_string(),
        count: 3,
    });
    let rendered = render_list_app(app, draw_tag)?;
    assert!(rendered.iter().any(|line| line.contains("release")));
    assert!(rendered.iter().any(|line| line.contains("3")));
    Ok(())
}

#[test]
fn loop_list_renderer_draws_invalid_loop_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(list_project_index());
    app.view = View::LoopList;
    app.supplement.loops.push(TuiLoopEntry {
        id: "LOOP-2026-06-06-002".to_string(),
        state: None,
        diagnostic: Some(Diagnostic::new(
            DiagnosticCode::E1201LoopStateInvalid,
            "bad loop state",
            ".govctl/loops/LOOP-2026-06-06-002/state.toml",
        )),
    });

    let rendered = render_list_app(app, draw_loop)?;

    assert!(rendered.iter().any(|line| line.contains("invalid")));
    assert!(rendered.iter().any(|line| line.contains("bad loop state")));
    Ok(())
}

fn render_list(
    view: View,
    draw: fn(&mut Frame, &mut App, Rect),
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut app = App::new(list_project_index());
    app.view = view;

    let (_, rendered) = render_app(110, 8, app, |frame, app| draw(frame, app, frame.area()))?;
    Ok(rendered)
}

fn render_list_app(
    app: App,
    draw: fn(&mut Frame, &mut App, Rect),
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let (_, rendered) = render_app(120, 10, app, |frame, app| draw(frame, app, frame.area()))?;
    Ok(rendered)
}

fn list_project_index() -> crate::model::ProjectIndex {
    project_index(
        vec![rfc(
            "RFC-0001",
            "RFC title",
            RfcStatus::Normative,
            RfcPhase::Impl,
            &["core"],
        )],
        vec![adr(
            "ADR-0001",
            "ADR title",
            AdrStatus::Accepted,
            &["design"],
        )],
        vec![work_item(
            "WI-2026-01-01-001",
            "Work title",
            WorkItemStatus::Active,
            &["cleanup"],
        )],
    )
}

fn loop_state() -> crate::diagnostic::DiagnosticResult<LoopState> {
    let work_id = "WI-2026-06-06-001";
    let mut dependencies = BTreeMap::new();
    dependencies.insert(work_id.to_string(), Vec::new());
    let mut state = LoopState::new(
        "LOOP-2026-06-06-001",
        vec![work_id.to_string()],
        vec![work_id.to_string()],
        dependencies,
    )?;
    state.loop_meta.next_action = crate::loop_state::LoopNextAction::Continue;
    Ok(state)
}

fn guard_entry() -> GuardEntry {
    GuardEntry {
        spec: GuardSpec {
            govctl: GuardMeta::new("GUARD-LIST", "Guard row"),
            check: GuardCheck {
                command: "cargo test".to_string(),
                timeout_secs: 30,
                pattern: None,
            },
        },
        path: "gov/guard/GUARD-LIST.toml".into(),
    }
}
