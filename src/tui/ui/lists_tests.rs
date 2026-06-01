use super::super::super::app::View;
use super::super::test_support::{adr, buffer_lines, project_index, rfc, work_item};
use super::*;
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};
use ratatui::{Terminal, backend::TestBackend};

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

fn render_list(
    view: View,
    draw: fn(&mut Frame, &mut App, Rect),
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = TestBackend::new(110, 8);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(list_project_index());
    app.view = view;

    terminal.draw(|frame| draw(frame, &mut app, frame.area()))?;

    Ok(buffer_lines(terminal.backend().buffer()))
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
