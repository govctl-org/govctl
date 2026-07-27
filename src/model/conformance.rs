use serde::{Deserialize, Serialize};

use super::{ClauseStatus, ConformanceEntry, ProjectIndex, RfcPhase, RfcStatus};

/// Conformance Case metadata section `[govctl]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceMeta {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Versioned RFC Clause binding owned by a Conformance Case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementBinding {
    #[serde(rename = "ref")]
    pub clause_ref: String,
    pub version: String,
}

/// Project-owned scenario locator and its declared trace edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceContent {
    pub path: String,
    pub selector: String,
    pub requirements: Vec<RequirementBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guards: Vec<String>,
}

/// Complete Conformance Case file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceSpec {
    pub govctl: ConformanceMeta,
    pub case: ConformanceContent,
}

/// Derived requirement applicability used by trace consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConformanceApplicability {
    Stale,
    Provisional,
    Candidate,
    Current,
}

impl std::fmt::Display for ConformanceApplicability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Stale => "stale",
            Self::Provisional => "provisional",
            Self::Candidate => "candidate",
            Self::Current => "current",
        };
        formatter.write_str(value)
    }
}

/// A requirement binding enriched with its derived applicability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConformanceTraceRequirement {
    #[serde(rename = "ref")]
    pub clause_ref: String,
    pub version: String,
    pub requirement_applicability: ConformanceApplicability,
}

/// A Conformance Case enriched with trace-query semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConformanceTraceCase {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub path: String,
    pub selector: String,
    pub requirements: Vec<ConformanceTraceRequirement>,
    pub guards: Vec<String>,
    pub requirement_applicability: ConformanceApplicability,
}

/// Derive the trace-query projection for a Conformance Case.
pub fn derive_conformance_trace(
    entry: &ConformanceEntry,
    index: &ProjectIndex,
) -> ConformanceTraceCase {
    let mut requirements = entry
        .spec
        .case
        .requirements
        .iter()
        .map(|binding| ConformanceTraceRequirement {
            clause_ref: binding.clause_ref.clone(),
            version: binding.version.clone(),
            requirement_applicability: requirement_applicability(binding, index),
        })
        .collect::<Vec<_>>();
    requirements.sort_by(|left, right| {
        left.clause_ref
            .cmp(&right.clause_ref)
            .then_with(|| left.version.cmp(&right.version))
    });

    let mut tags = entry.meta().tags.clone();
    tags.sort();
    tags.dedup();
    let mut guards = entry.spec.case.guards.clone();
    guards.sort();
    guards.dedup();
    let requirement_applicability = requirements
        .iter()
        .map(|requirement| requirement.requirement_applicability)
        .min()
        .unwrap_or(ConformanceApplicability::Stale);

    ConformanceTraceCase {
        id: entry.meta().id.clone(),
        title: entry.meta().title.clone(),
        tags,
        path: entry.spec.case.path.clone(),
        selector: entry.spec.case.selector.clone(),
        requirements,
        guards,
        requirement_applicability,
    }
}

fn requirement_applicability(
    binding: &RequirementBinding,
    index: &ProjectIndex,
) -> ConformanceApplicability {
    let Some((rfc_id, clause_id)) = binding.clause_ref.split_once(':') else {
        return ConformanceApplicability::Stale;
    };
    let Some(rfc) = index.rfcs.iter().find(|entry| entry.rfc.rfc_id == rfc_id) else {
        return ConformanceApplicability::Stale;
    };
    let Some(clause) = rfc
        .clauses
        .iter()
        .find(|entry| entry.spec.clause_id == clause_id)
    else {
        return ConformanceApplicability::Stale;
    };
    if clause.spec.status != ClauseStatus::Active || binding.version != rfc.rfc.version {
        return ConformanceApplicability::Stale;
    }

    match (rfc.rfc.status, rfc.rfc.phase, clause.spec.since.is_some()) {
        (RfcStatus::Draft, _, _) => ConformanceApplicability::Provisional,
        (RfcStatus::Normative, RfcPhase::Spec, true) => ConformanceApplicability::Candidate,
        (RfcStatus::Normative, RfcPhase::Impl | RfcPhase::Test | RfcPhase::Stable, true) => {
            ConformanceApplicability::Current
        }
        _ => ConformanceApplicability::Stale,
    }
}
