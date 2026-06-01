use crate::model::{AdrStatus, ClauseStatus, ProjectIndex, RfcStatus};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactRefState {
    Active,
    Outdated(&'static str),
}

pub(crate) fn artifact_ref_ids(index: &ProjectIndex) -> HashSet<String> {
    artifact_ref_states(index).into_keys().collect()
}

pub(crate) fn artifact_ref_states(index: &ProjectIndex) -> HashMap<String, ArtifactRefState> {
    let mut known = HashMap::new();

    for rfc in &index.rfcs {
        let rfc_state = match rfc.rfc.status {
            RfcStatus::Deprecated => ArtifactRefState::Outdated("deprecated"),
            _ => ArtifactRefState::Active,
        };
        known.insert(rfc.rfc.rfc_id.clone(), rfc_state);

        for clause in &rfc.clauses {
            let clause_id = format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id);
            let clause_state = match clause.spec.status {
                ClauseStatus::Superseded => ArtifactRefState::Outdated("superseded"),
                ClauseStatus::Deprecated => ArtifactRefState::Outdated("deprecated"),
                ClauseStatus::Active => {
                    if rfc.rfc.status == RfcStatus::Deprecated {
                        ArtifactRefState::Outdated("RFC deprecated")
                    } else {
                        ArtifactRefState::Active
                    }
                }
            };
            known.insert(clause_id, clause_state);
        }
    }

    for adr in &index.adrs {
        let adr_state = match adr.meta().status {
            AdrStatus::Superseded => ArtifactRefState::Outdated("superseded"),
            _ => ArtifactRefState::Active,
        };
        known.insert(adr.meta().id.clone(), adr_state);
    }

    for work in &index.work_items {
        known.insert(work.meta().id.clone(), ArtifactRefState::Active);
    }

    known
}
