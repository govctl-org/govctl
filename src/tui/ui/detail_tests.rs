use super::super::super::app::{App, View};
use super::super::test_support::{adr, clause, project_index, render_app, rfc, work_item};
use super::*;
use crate::loop_state::{LoopState, LoopWorkItemStatus};
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};
use crate::tui::data::TuiLoopEntry;
use std::collections::BTreeMap;

#[test]
fn detail_viewport_footer_status_clamps_scroll() {
    let viewport = DetailViewport::new(4);
    let mut scroll = 8;

    assert_eq!(viewport.footer_status(&mut scroll), "Scroll 4/4");
    assert_eq!(scroll, 3);
}

#[test]
fn detail_renderers_draw_expected_content() -> Result<(), Box<dyn std::error::Error>> {
    let rendered = render_detail(View::RfcDetail(0), |frame, app, area| {
        draw_rfc(frame, app, area, 0);
    })?;
    assert!(rendered.iter().any(|line| line.contains("RFC-0001")));
    assert!(rendered.iter().any(|line| line.contains("RFC title")));
    assert!(rendered.iter().any(|line| line.contains("C-TEST")));

    let rendered = render_detail(View::AdrDetail(0), |frame, app, area| {
        draw_adr(frame, app, area, 0);
    })?;
    assert!(rendered.iter().any(|line| line.contains("ADR-0001")));
    assert!(rendered.iter().any(|line| line.contains("ADR title")));

    let rendered = render_detail(View::WorkDetail(0), |frame, app, area| {
        draw_work(frame, app, area, 0);
    })?;
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("WI-2026-01-01-001"))
    );
    assert!(rendered.iter().any(|line| line.contains("Work title")));

    let rendered = render_clause_detail()?;
    assert!(rendered.iter().any(|line| line.contains("C-TEST")));
    assert!(rendered.iter().any(|line| line.contains("Clause text")));

    Ok(())
}

#[test]
fn loop_detail_renders_dag_and_inspector_on_narrow_viewport()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(detail_project_index());
    app.view = View::LoopDetail(0);
    app.loop_selected = 1;
    app.supplement.loops.push(TuiLoopEntry {
        id: "LOOP-2026-06-06-001".to_string(),
        state: Some(sample_loop_state()?),
        diagnostic: None,
    });

    let (_, rendered) = render_app(72, 24, app, |frame, app| {
        draw_loop(frame, app, frame.area(), 0);
    })?;

    assert!(rendered.iter().any(|line| line.contains("Dependency DAG")));
    assert!(rendered.iter().any(|line| line.contains("Selected Work")));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("WI-2026-06-06-002"))
    );
    assert!(rendered.iter().any(|line| line.contains("Status:")));
    Ok(())
}

fn render_detail(
    view: View,
    mut draw: impl FnMut(&mut Frame, &mut App, Rect),
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut app = App::new(detail_project_index());
    app.view = view;

    let (_, rendered) = render_app(110, 12, app, |frame, app| draw(frame, app, frame.area()))?;
    Ok(rendered)
}

fn render_clause_detail() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut app = App::new(detail_project_index());
    app.view = View::ClauseDetail(0, 0);

    let (_, rendered) = render_app(110, 12, app, |frame, app| {
        draw_clause(frame, app, frame.area(), 0, 0);
    })?;
    Ok(rendered)
}

fn detail_project_index() -> crate::model::ProjectIndex {
    let mut rfc = rfc(
        "RFC-0001",
        "RFC title",
        RfcStatus::Normative,
        RfcPhase::Impl,
        &["core"],
    );
    rfc.clauses
        .push(clause("C-TEST", "Clause title", "Clause text"));

    project_index(
        vec![rfc],
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

fn sample_loop_state() -> crate::diagnostic::DiagnosticResult<LoopState> {
    let mut dependencies = BTreeMap::new();
    dependencies.insert("WI-2026-06-06-001".to_string(), vec![]);
    dependencies.insert(
        "WI-2026-06-06-002".to_string(),
        vec!["WI-2026-06-06-001".to_string()],
    );
    let mut state = LoopState::new(
        "LOOP-2026-06-06-001",
        vec!["WI-2026-06-06-002".to_string()],
        vec![
            "WI-2026-06-06-001".to_string(),
            "WI-2026-06-06-002".to_string(),
        ],
        dependencies,
    )?;
    state.set_item_status("WI-2026-06-06-002", LoopWorkItemStatus::Active)?;
    Ok(state)
}
