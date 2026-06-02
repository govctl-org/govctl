use super::super::app::{App, View};
use super::test_support::{adr, buffer_lines, clause, project_index, rfc, work_item};
use super::*;
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn draw_renders_chrome_and_help_overlay() -> Result<(), Box<dyn std::error::Error>> {
    let backend = TestBackend::new(100, 18);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(project_index(
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
    ));
    app.view = View::RfcList;
    app.show_help = true;

    terminal.draw(|frame| draw(frame, &mut app))?;

    let rendered = buffer_lines(terminal.backend().buffer());
    assert!(rendered.iter().any(|line| line.contains("govctl")));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("Dashboard > RFCs"))
    );
    assert!(rendered.iter().any(|line| line.contains("Shown 1/1")));
    assert!(rendered.iter().any(|line| line.contains("Global")));
    assert!(rendered.iter().any(|line| line.contains("List")));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("Enter  View detail"))
    );
    Ok(())
}

#[test]
fn draw_clamps_detail_scroll_and_renders_footer_status() -> Result<(), Box<dyn std::error::Error>> {
    for view in [
        View::AdrDetail(0),
        View::WorkDetail(0),
        View::ClauseDetail(0, 0),
    ] {
        let (scroll, rendered) = render_scrolled_detail(view)?;

        assert!(scroll < u16::MAX);
        assert!(rendered.iter().any(|line| line.contains("Scroll ")));
    }

    Ok(())
}

fn render_scrolled_detail(view: View) -> Result<(u16, Vec<String>), Box<dyn std::error::Error>> {
    let backend = TestBackend::new(100, 18);
    let mut terminal = Terminal::new(backend)?;
    let mut rfc = rfc(
        "RFC-0001",
        "RFC title",
        RfcStatus::Normative,
        RfcPhase::Impl,
        &["core"],
    );
    rfc.clauses
        .push(clause("C-TEST", "Clause title", "Clause text"));

    let mut app = App::new(project_index(
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
    ));
    app.view = view;
    app.scroll = u16::MAX;

    terminal.draw(|frame| draw(frame, &mut app))?;

    Ok((app.scroll, buffer_lines(terminal.backend().buffer())))
}
