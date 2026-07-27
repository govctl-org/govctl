use super::super::app::{App, View};
use super::test_support::{adr, clause, project_index, render_app, rfc, work_item};
use super::*;
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};

#[test]
fn draw_renders_chrome_and_help_overlay() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = ui_app();
    app.view = View::RfcList;
    app.show_help = true;

    let (_, rendered) = render_app(100, 18, app, draw)?;
    assert!(rendered.iter().any(|line| line.contains("GOVCTL")));
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("DASHBOARD > RFCS"))
    );
    assert!(rendered.iter().any(|line| line.contains("SEL 1 / 1")));
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
fn narrow_list_draws_filter_strip_result_context_and_records()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = ui_app();
    app.view = View::RfcList;
    app.enter_filter_mode();

    let (_, rendered) = render_app(72, 18, app, draw)?;
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("FILTER // EDITING"))
    );
    assert!(rendered.iter().any(|line| line.contains("MATCH SET")));
    assert!(rendered.iter().any(|line| line.contains("/▏")));
    assert!(rendered.iter().any(|line| line.contains("RFC INDEX")));
    assert!(rendered.iter().any(|line| line.contains("1 RECORD")));
    assert!(rendered.iter().any(|line| line.contains("[q] Quit")));
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

    let (app, rendered) = render_app(100, 18, app, draw)?;
    Ok((app.scroll, rendered))
}

fn ui_app() -> App {
    App::new(project_index(
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
    ))
}
