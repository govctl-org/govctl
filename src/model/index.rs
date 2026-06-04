use std::path::PathBuf;

use super::{
    AdrMeta, AdrSpec, ClauseSpec, GuardMeta, GuardSpec, RfcSpec, WorkItemMeta, WorkItemSpec,
};

/// Loaded RFC with all its clauses
#[derive(Debug, Clone)]
pub struct RfcIndex {
    pub rfc: RfcSpec,
    pub clauses: Vec<ClauseEntry>,
    pub path: PathBuf,
}

/// Clause with its path
#[derive(Debug, Clone)]
pub struct ClauseEntry {
    pub spec: ClauseSpec,
    pub path: PathBuf,
}

/// Loaded ADR with full spec
#[derive(Debug, Clone)]
pub struct AdrEntry {
    pub spec: AdrSpec,
    pub path: PathBuf,
}

impl AdrEntry {
    /// Convenience accessor for metadata
    pub fn meta(&self) -> &AdrMeta {
        &self.spec.govctl
    }
}

/// Loaded Work Item with full spec
#[derive(Debug, Clone)]
pub struct WorkItemEntry {
    pub spec: WorkItemSpec,
    pub path: PathBuf,
}

impl WorkItemEntry {
    /// Convenience accessor for metadata
    pub fn meta(&self) -> &WorkItemMeta {
        &self.spec.govctl
    }
}

/// Loaded Verification Guard with full spec.
#[derive(Debug, Clone)]
pub struct GuardEntry {
    pub spec: GuardSpec,
    pub path: PathBuf,
}

impl GuardEntry {
    pub fn meta(&self) -> &GuardMeta {
        &self.spec.govctl
    }
}

/// Full project index
#[derive(Debug, Clone, Default)]
pub struct ProjectIndex {
    pub rfcs: Vec<RfcIndex>,
    pub adrs: Vec<AdrEntry>,
    pub work_items: Vec<WorkItemEntry>,
}

impl ProjectIndex {
    /// Iterate over all clauses across all RFCs
    pub fn iter_clauses(&self) -> impl Iterator<Item = (&RfcIndex, &ClauseEntry)> {
        self.rfcs
            .iter()
            .flat_map(|rfc| rfc.clauses.iter().map(move |c| (rfc, c)))
    }
}
