use super::super::test_support::{adr, project_index, render_app, rfc, work_item};
use super::*;
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};

#[test]
fn wide_dashboard_draws_control_plane_sections_and_state() -> Result<(), Box<dyn std::error::Error>>
{
    let app = App::new(dashboard_project_index());

    let (_, rendered) = render_app(120, 24, app, |frame, app| draw(frame, app, frame.area()))?;
    for marker in [
        "LIFECYCLE MATRIX",
        "OPERATIONS",
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
    ] {
        assert!(
            rendered.iter().any(|line| line.contains(marker)),
            "missing {marker}: {rendered:?}"
        );
    }
    Ok(())
}

#[test]
fn narrow_dashboard_uses_compact_readable_layout() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new(dashboard_project_index());

    let (_, rendered) = render_app(72, 18, app, |frame, app| draw(frame, app, frame.area()))?;
    for marker in [
        "LIFECYCLE MATRIX",
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
    project_index(
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
    )
}
