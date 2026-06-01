use super::super::super::app::{App, View};
use super::super::test_support::{adr, buffer_lines, clause, project_index, rfc, work_item};
use super::*;
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};
use ratatui::{Terminal, backend::TestBackend};

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

fn render_detail(
    view: View,
    mut draw: impl FnMut(&mut Frame, &mut App, Rect),
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = TestBackend::new(110, 12);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(detail_project_index());
    app.view = view;

    terminal.draw(|frame| {
        draw(frame, &mut app, frame.area());
    })?;

    Ok(buffer_lines(terminal.backend().buffer()))
}

fn render_clause_detail() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = TestBackend::new(110, 12);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(detail_project_index());
    app.view = View::ClauseDetail(0, 0);

    terminal.draw(|frame| {
        draw_clause(frame, &mut app, frame.area(), 0, 0);
    })?;

    Ok(buffer_lines(terminal.backend().buffer()))
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
