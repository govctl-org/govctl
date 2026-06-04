use super::super::app::App;
use crate::model::{
    AdrContent, AdrEntry, AdrMeta, AdrSpec, AdrStatus, ClauseEntry, ClauseKind, ClauseSpec,
    ClauseStatus, ProjectIndex, RfcIndex, RfcPhase, RfcSpec, RfcStatus, WorkItemContent,
    WorkItemEntry, WorkItemMeta, WorkItemSpec, WorkItemStatus, WorkItemVerification,
};
use ratatui::buffer::Buffer;
use ratatui::{Terminal, backend::TestBackend, prelude::*};
use std::path::PathBuf;

pub(super) fn buffer_lines(buffer: &Buffer) -> Vec<String> {
    let width = buffer.area().width as usize;
    buffer
        .content()
        .chunks(width)
        .map(|row| {
            let mut line = String::new();
            for cell in row {
                line.push_str(cell.symbol());
            }
            line
        })
        .collect()
}

pub(super) fn render_app(
    width: u16,
    height: u16,
    mut app: App,
    mut draw: impl FnMut(&mut Frame, &mut App),
) -> Result<(App, Vec<String>), Box<dyn std::error::Error>> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| draw(frame, &mut app))?;

    let rendered = buffer_lines(terminal.backend().buffer());
    Ok((app, rendered))
}

pub(super) fn project_index(
    rfcs: Vec<RfcIndex>,
    adrs: Vec<AdrEntry>,
    work_items: Vec<WorkItemEntry>,
) -> ProjectIndex {
    ProjectIndex {
        rfcs,
        adrs,
        work_items,
    }
}

pub(super) fn rfc(
    id: &str,
    title: &str,
    status: RfcStatus,
    phase: RfcPhase,
    tags: &[&str],
) -> RfcIndex {
    RfcIndex {
        rfc: RfcSpec {
            rfc_id: id.to_string(),
            title: title.to_string(),
            version: "0.1.0".to_string(),
            status,
            phase,
            owners: vec![],
            created: "2026-01-01".to_string(),
            updated: None,
            supersedes: None,
            refs: vec![],
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            sections: vec![],
            changelog: vec![],
            signature: None,
        },
        clauses: vec![],
        path: PathBuf::from(format!("gov/rfc/{id}.toml")),
    }
}

pub(super) fn clause(id: &str, title: &str, text: &str) -> ClauseEntry {
    ClauseEntry {
        spec: ClauseSpec {
            clause_id: id.to_string(),
            title: title.to_string(),
            kind: ClauseKind::Normative,
            status: ClauseStatus::Active,
            text: text.to_string(),
            anchors: vec![],
            superseded_by: None,
            since: None,
            tags: vec![],
        },
        path: PathBuf::from(format!("gov/rfc/clauses/{id}.toml")),
    }
}

pub(super) fn adr(id: &str, title: &str, status: AdrStatus, tags: &[&str]) -> AdrEntry {
    let mut meta = AdrMeta::new(id, title, status, "2026-01-01");
    meta.tags = tags.iter().map(|tag| tag.to_string()).collect();

    AdrEntry {
        spec: AdrSpec {
            govctl: meta,
            content: AdrContent::default(),
        },
        path: PathBuf::from(format!("gov/adr/{id}.toml")),
    }
}

pub(super) fn work_item(
    id: &str,
    title: &str,
    status: WorkItemStatus,
    tags: &[&str],
) -> WorkItemEntry {
    let mut meta = WorkItemMeta::new(id, title, status);
    meta.tags = tags.iter().map(|tag| tag.to_string()).collect();

    WorkItemEntry {
        spec: WorkItemSpec {
            govctl: meta,
            content: WorkItemContent::default(),
            verification: WorkItemVerification::default(),
        },
        path: PathBuf::from(format!("gov/work/{id}.toml")),
    }
}
