use crate::model::{
    AdrContent, AdrEntry, AdrMeta, AdrSpec, AdrStatus, ProjectIndex, RfcIndex, RfcPhase, RfcSpec,
    RfcStatus, WorkItemContent, WorkItemEntry, WorkItemMeta, WorkItemSpec, WorkItemStatus,
    WorkItemVerification,
};
use ratatui::buffer::Buffer;
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

pub(super) fn adr(id: &str, title: &str, status: AdrStatus, tags: &[&str]) -> AdrEntry {
    AdrEntry {
        spec: AdrSpec {
            govctl: AdrMeta {
                schema: 1,
                id: id.to_string(),
                title: title.to_string(),
                status,
                date: "2026-01-01".to_string(),
                superseded_by: None,
                refs: vec![],
                tags: tags.iter().map(|tag| tag.to_string()).collect(),
            },
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
    WorkItemEntry {
        spec: WorkItemSpec {
            govctl: WorkItemMeta {
                schema: 1,
                id: id.to_string(),
                title: title.to_string(),
                status,
                created: None,
                started: None,
                completed: None,
                refs: vec![],
                depends_on: vec![],
                tags: tags.iter().map(|tag| tag.to_string()).collect(),
            },
            content: WorkItemContent::default(),
            verification: WorkItemVerification::default(),
        },
        path: PathBuf::from(format!("gov/work/{id}.toml")),
    }
}
