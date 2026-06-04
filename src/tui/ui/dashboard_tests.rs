use super::super::test_support::{adr, project_index, render_app, rfc, work_item};
use super::*;
use crate::model::{AdrStatus, RfcPhase, RfcStatus, WorkItemStatus};

#[test]
fn dashboard_draws_summary_counts() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new(dashboard_project_index());

    let (_, rendered) = render_app(90, 8, app, |frame, app| draw(frame, app, frame.area()))?;
    assert!(rendered.iter().any(|line| line.contains("Draft:      1")));
    assert!(rendered.iter().any(|line| line.contains("Normative:  1")));
    assert!(rendered.iter().any(|line| line.contains("Proposed:   1")));
    assert!(rendered.iter().any(|line| line.contains("Accepted:   1")));
    assert!(rendered.iter().any(|line| line.contains("Queue:  1")));
    assert!(rendered.iter().any(|line| line.contains("Active: 1")));
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
        ],
    )
}
