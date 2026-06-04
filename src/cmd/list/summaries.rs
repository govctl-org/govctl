use super::output::truncate_chars;
use crate::model::{AdrEntry, ClauseEntry, GuardEntry, RfcIndex, WorkItemEntry};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct RfcSummary {
    id: String,
    version: String,
    status: String,
    phase: String,
    title: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    amended: bool,
}

impl RfcSummary {
    pub(super) fn from_entry(rfc: &RfcIndex) -> Self {
        let amended = crate::signature::is_rfc_amended(rfc);
        Self {
            id: if amended {
                format!("{}*", rfc.rfc.rfc_id)
            } else {
                rfc.rfc.rfc_id.clone()
            },
            version: rfc.rfc.version.clone(),
            status: rfc.rfc.status.as_ref().to_string(),
            phase: rfc.rfc.phase.as_ref().to_string(),
            title: rfc.rfc.title.clone(),
            amended,
        }
    }

    pub(super) fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.version.clone(),
            self.status.clone(),
            self.phase.clone(),
            self.title.clone(),
        ]
    }
}

#[derive(Serialize)]
pub(super) struct ClauseSummary {
    id: String,
    rfc_id: String,
    kind: String,
    status: String,
    title: String,
}

impl ClauseSummary {
    pub(super) fn from_entry(rfc_id: &str, clause: &ClauseEntry) -> Self {
        Self {
            id: clause.spec.clause_id.clone(),
            rfc_id: rfc_id.to_string(),
            kind: clause.spec.kind.as_ref().to_string(),
            status: clause.spec.status.as_ref().to_string(),
            title: clause.spec.title.clone(),
        }
    }

    pub(super) fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.rfc_id.clone(),
            self.kind.clone(),
            self.status.clone(),
            self.title.clone(),
        ]
    }
}

#[derive(Serialize)]
pub(super) struct AdrSummary {
    id: String,
    status: String,
    date: String,
    title: String,
}

impl AdrSummary {
    pub(super) fn from_entry(adr: &AdrEntry) -> Self {
        Self {
            id: adr.meta().id.clone(),
            status: adr.meta().status.as_ref().to_string(),
            date: adr.meta().date.clone(),
            title: adr.meta().title.clone(),
        }
    }

    pub(super) fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.status.clone(),
            self.date.clone(),
            self.title.clone(),
        ]
    }
}

#[derive(Serialize)]
pub(super) struct GuardSummary {
    id: String,
    title: String,
    command: String,
}

impl GuardSummary {
    pub(super) fn from_entry(guard: &GuardEntry) -> Self {
        Self {
            id: guard.meta().id.clone(),
            title: guard.meta().title.clone(),
            command: guard.spec.check.command.clone(),
        }
    }

    pub(super) fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            truncate_chars(&self.command, 50),
        ]
    }
}

#[derive(Serialize)]
pub(super) struct WorkItemSummary {
    id: String,
    status: String,
    title: String,
}

impl WorkItemSummary {
    pub(super) fn from_entry(item: &WorkItemEntry) -> Self {
        Self {
            id: item.meta().id.clone(),
            status: item.meta().status.as_ref().to_string(),
            title: item.meta().title.clone(),
        }
    }

    pub(super) fn row(&self) -> Vec<String> {
        vec![self.id.clone(), self.status.clone(), self.title.clone()]
    }
}
