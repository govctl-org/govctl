use crate::artifact_catalog::{CatalogKind, CatalogRecord};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticResult};
use crate::load::{load_clause, load_rfc};
use crate::model::{AdrEntry, ClauseEntry, ConformanceEntry, GuardEntry, RfcIndex, WorkItemEntry};
use crate::parse::{load_adr, load_conformance_case, load_guard, load_work_item};

#[derive(Debug, Clone)]
pub(super) struct SearchDocument {
    pub(super) kind: CatalogKind,
    pub(super) id: String,
    pub(super) title: String,
    pub(super) path: String,
    pub(super) tags: Vec<String>,
    pub(super) status: Option<String>,
    pub(super) body: String,
}

pub(super) fn build(config: &Config, record: &CatalogRecord) -> DiagnosticResult<SearchDocument> {
    let path = record.absolute_path(config);
    match record.kind {
        CatalogKind::Rfc => {
            let entry = load_rfc(config, &path).map_err(Diagnostic::from)?;
            Ok(rfc_document(config, &entry))
        }
        CatalogKind::Clause => {
            let entry = load_clause(config, &path).map_err(Diagnostic::from)?;
            Ok(clause_document(config, record, &entry))
        }
        CatalogKind::Adr => {
            let entry = load_adr(config, &path)?;
            Ok(adr_document(config, &entry))
        }
        CatalogKind::Work => {
            let entry = load_work_item(config, &path)?;
            Ok(work_document(config, &entry))
        }
        CatalogKind::Guard => {
            let entry = load_guard(config, &path)?;
            Ok(guard_document(config, &entry))
        }
        CatalogKind::Conformance => {
            let entry = load_conformance_case(config, &path)?;
            Ok(conformance_document(config, &entry))
        }
    }
}

fn rfc_document(config: &Config, entry: &RfcIndex) -> SearchDocument {
    let rfc = &entry.rfc;
    let mut parts = vec![
        rfc.rfc_id.clone(),
        rfc.title.clone(),
        rfc.version.clone(),
        rfc.status.as_ref().to_string(),
        rfc.phase.as_ref().to_string(),
    ];
    parts.extend(rfc.owners.iter().cloned());
    parts.extend(rfc.refs.iter().cloned());
    parts.extend(rfc.tags.iter().cloned());
    for section in &rfc.sections {
        parts.push(section.title.clone());
        parts.extend(section.clauses.iter().cloned());
    }
    for changelog in &rfc.changelog {
        parts.push(changelog.version.clone());
        parts.push(changelog.date.clone());
        if let Some(notes) = &changelog.notes {
            parts.push(notes.clone());
        }
        parts.extend(changelog.added.iter().cloned());
        parts.extend(changelog.changed.iter().cloned());
        parts.extend(changelog.deprecated.iter().cloned());
        parts.extend(changelog.removed.iter().cloned());
        parts.extend(changelog.fixed.iter().cloned());
        parts.extend(changelog.security.iter().cloned());
    }
    SearchDocument {
        kind: CatalogKind::Rfc,
        id: rfc.rfc_id.clone(),
        title: rfc.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: rfc.tags.clone(),
        status: Some(format!("{}/{}", rfc.status.as_ref(), rfc.phase.as_ref())),
        body: join_parts(parts),
    }
}

fn clause_document(config: &Config, record: &CatalogRecord, entry: &ClauseEntry) -> SearchDocument {
    let clause = &entry.spec;
    let mut parts = vec![
        record.id.clone(),
        clause.clause_id.clone(),
        clause.title.clone(),
        clause.kind.as_ref().to_string(),
        clause.status.as_ref().to_string(),
        clause.text.clone(),
    ];
    parts.extend(clause.anchors.iter().cloned());
    parts.extend(clause.tags.iter().cloned());
    if let Some(since) = &clause.since {
        parts.push(since.clone());
    }
    if let Some(superseded_by) = &clause.superseded_by {
        parts.push(superseded_by.clone());
    }
    SearchDocument {
        kind: CatalogKind::Clause,
        id: record.id.clone(),
        title: clause.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: clause.tags.clone(),
        status: Some(clause.status.as_ref().to_string()),
        body: join_parts(parts),
    }
}

fn adr_document(config: &Config, entry: &AdrEntry) -> SearchDocument {
    let meta = &entry.spec.govctl;
    let content = &entry.spec.content;
    let mut parts = vec![
        meta.id.clone(),
        meta.title.clone(),
        meta.status.as_ref().to_string(),
        meta.date.clone(),
        content.context.clone(),
        content.decision.clone(),
        content.consequences.clone(),
    ];
    parts.extend(meta.refs.iter().cloned());
    parts.extend(meta.tags.iter().cloned());
    if let Some(superseded_by) = &meta.superseded_by {
        parts.push(superseded_by.clone());
    }
    for alternative in &content.alternatives {
        parts.push(alternative.text.clone());
        parts.push(alternative.status.as_ref().to_string());
        parts.extend(alternative.pros.iter().cloned());
        parts.extend(alternative.cons.iter().cloned());
        if let Some(reason) = &alternative.rejection_reason {
            parts.push(reason.clone());
        }
    }
    SearchDocument {
        kind: CatalogKind::Adr,
        id: meta.id.clone(),
        title: meta.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: meta.tags.clone(),
        status: Some(meta.status.as_ref().to_string()),
        body: join_parts(parts),
    }
}

fn work_document(config: &Config, entry: &WorkItemEntry) -> SearchDocument {
    let meta = &entry.spec.govctl;
    let content = &entry.spec.content;
    let mut parts = vec![
        meta.id.clone(),
        meta.title.clone(),
        meta.status.as_ref().to_string(),
        content.description.clone(),
    ];
    parts.extend(meta.refs.iter().cloned());
    parts.extend(meta.depends_on.iter().cloned());
    parts.extend(meta.tags.iter().cloned());
    for item in &content.acceptance_criteria {
        parts.push(item.text.clone());
        parts.push(item.status.as_ref().to_string());
        parts.push(item.category.as_ref().to_string());
    }
    parts.extend(content.notes.iter().cloned());
    SearchDocument {
        kind: CatalogKind::Work,
        id: meta.id.clone(),
        title: meta.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: meta.tags.clone(),
        status: Some(meta.status.as_ref().to_string()),
        body: join_parts(parts),
    }
}

fn guard_document(config: &Config, entry: &GuardEntry) -> SearchDocument {
    let meta = &entry.spec.govctl;
    let mut parts = vec![
        meta.id.clone(),
        meta.title.clone(),
        entry.spec.check.command.clone(),
    ];
    parts.extend(meta.refs.iter().cloned());
    parts.extend(meta.tags.iter().cloned());
    if let Some(pattern) = &entry.spec.check.pattern {
        parts.push(pattern.clone());
    }
    SearchDocument {
        kind: CatalogKind::Guard,
        id: meta.id.clone(),
        title: meta.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: meta.tags.clone(),
        status: None,
        body: join_parts(parts),
    }
}

fn conformance_document(config: &Config, entry: &ConformanceEntry) -> SearchDocument {
    let meta = entry.meta();
    let mut parts = vec![
        meta.id.clone(),
        meta.title.clone(),
        entry.spec.case.path.clone(),
        entry.spec.case.selector.clone(),
    ];
    parts.extend(meta.tags.iter().cloned());
    parts.extend(entry.spec.case.guards.iter().cloned());
    for requirement in &entry.spec.case.requirements {
        parts.push(requirement.clause_ref.clone());
        parts.push(requirement.version.clone());
    }
    SearchDocument {
        kind: CatalogKind::Conformance,
        id: meta.id.clone(),
        title: meta.title.clone(),
        path: config.display_path(&entry.path).display().to_string(),
        tags: meta.tags.clone(),
        status: None,
        body: join_parts(parts),
    }
}

fn join_parts(parts: Vec<String>) -> String {
    parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
