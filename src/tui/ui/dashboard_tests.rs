use super::super::test_support::{
    adr, conformance_case, project_index, render_app, rfc, work_item,
};
use super::*;
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};

#[test]
fn wide_dashboard_draws_control_plane_sections_and_state() -> Result<(), Box<dyn std::error::Error>>
{
    let app = App::new(dashboard_project_index());

    let (_, rendered) = render_app(120, 24, app, |frame, app| draw(frame, app, frame.area()))?;
    for marker in [
        "LIFECYCLE MATRIX",
        "EXECUTION",
        "HEALTH",
        "GOVERNANCE INDEX",
        "draft",
        "normative",
        "proposed",
        "accepted",
        "rejected",
        "superseded",
        "queue",
        "active",
        "cancelled",
        "NOMINAL",
        "[s]",
        "[9]",
        "[t]",
        "[x]",
        "CASE TRACE",
    ] {
        assert!(
            rendered.iter().any(|line| line.contains(marker)),
            "missing {marker}: {rendered:?}"
        );
    }

    let Some(rfc) = rendered
        .iter()
        .find(|line| line.contains("RFC") && line.contains("draft"))
    else {
        return Err("missing RFC lifecycle row".into());
    };
    let Some(adr) = rendered
        .iter()
        .find(|line| line.contains("ADR") && line.contains("proposed"))
    else {
        return Err("missing ADR lifecycle row".into());
    };
    let Some(work) = rendered
        .iter()
        .find(|line| line.contains("WORK") && line.contains("queue"))
    else {
        return Err("missing Work lifecycle row".into());
    };
    assert_eq!(rfc.find("draft"), adr.find("proposed"));
    assert_eq!(rfc.find("draft"), work.find("queue"));
    assert_eq!(rfc.find("normative"), adr.find("accepted"));
    assert_eq!(rfc.find("normative"), work.find("active"));
    assert_eq!(rfc.find("deprecated"), adr.find("superseded"));
    assert_eq!(rfc.find("deprecated"), work.find("cancelled"));

    let Some(first_index_row) = rendered.iter().find(|line| line.contains("RFC INDEX")) else {
        return Err("missing first governance index row".into());
    };
    let Some(second_index_row) = rendered.iter().find(|line| line.contains("ADR INDEX")) else {
        return Err("missing second governance index row".into());
    };
    assert_eq!(
        first_index_row.find("RFC INDEX"),
        second_index_row.find("ADR INDEX")
    );
    assert_eq!(
        first_index_row.find("CLAUSE INDEX"),
        second_index_row.find("WORK QUEUE")
    );
    Ok(())
}

#[test]
fn narrow_dashboard_uses_compact_readable_layout() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new(dashboard_project_index());

    let (_, rendered) = render_app(72, 18, app, |frame, app| draw(frame, app, frame.area()))?;
    for marker in [
        "LIFECYCLE MATRIX",
        "EXECUTION",
        "HEALTH",
        "GOVERNANCE INDEX",
        "deprecated",
        "rejected",
        "superseded",
        "cancelled",
        "NOMINAL",
        "[r]",
        "[d]",
    ] {
        assert!(
            rendered.iter().any(|line| line.contains(marker)),
            "missing {marker}: {rendered:?}"
        );
    }
    Ok(())
}

fn dashboard_project_index() -> crate::model::ProjectIndex {
    let mut index = project_index(
        vec![
            rfc(
                "RFC-0001",
                "RFC-0001",
                RfcStatus::Draft,
                RfcPhase::Spec,
                &[],
            ),
            rfc(
                "RFC-0002",
                "RFC-0002",
                RfcStatus::Normative,
                RfcPhase::Spec,
                &[],
            ),
        ],
        vec![
            adr("ADR-0001", "ADR-0001", AdrStatus::Proposed, &[]),
            adr("ADR-0002", "ADR-0002", AdrStatus::Accepted, &[]),
            adr("ADR-0003", "ADR-0003", AdrStatus::Rejected, &[]),
            adr("ADR-0004", "ADR-0004", AdrStatus::Superseded, &[]),
        ],
        vec![
            work_item(
                "WI-2026-01-01-001",
                "WI-2026-01-01-001",
                WorkItemStatus::Queue,
                &[],
            ),
            work_item(
                "WI-2026-01-01-002",
                "WI-2026-01-01-002",
                WorkItemStatus::Active,
                &[],
            ),
            work_item(
                "WI-2026-01-01-003",
                "WI-2026-01-01-003",
                WorkItemStatus::Cancelled,
                &[],
            ),
        ],
    );
    index.conformance_cases.push(conformance_case(
        "CONF-DASHBOARD",
        "Dashboard case",
        "RFC-0002:C-DASHBOARD",
        "0.1.0",
        &["testing"],
    ));
    index
}
