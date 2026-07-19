use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::ProjectIndex;
use regex::Regex;

pub(super) fn clause_deletion_referrers(
    config: &Config,
    index: &ProjectIndex,
    clause_id: &str,
) -> DiagnosticResult<Vec<String>> {
    let mut referrers = project_referrers(index, clause_id, None, false);
    referrers.extend(clause_supersession_referrers(index, clause_id));
    let inline_re = Regex::new(&config.source_scan.pattern).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            format!("Invalid source_scan.pattern regex: {err}"),
            config
                .display_path(&config.gov_root.join("config.toml"))
                .display()
                .to_string(),
        )
    })?;
    referrers.extend(inline_clause_referrers(index, &inline_re, clause_id));
    referrers.extend(guard_referrers(config, clause_id)?);
    referrers.sort();
    referrers.dedup();
    Ok(referrers)
}

fn clause_supersession_referrers(index: &ProjectIndex, target_id: &str) -> Vec<String> {
    index
        .rfcs
        .iter()
        .flat_map(|rfc| {
            rfc.clauses.iter().filter_map(move |clause| {
                let replacement = clause.spec.superseded_by.as_deref()?;
                let qualified = if replacement.contains(':') {
                    replacement.to_string()
                } else {
                    format!("{}:{replacement}", rfc.rfc.rfc_id)
                };
                (qualified == target_id)
                    .then(|| format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id))
            })
        })
        .collect()
}

fn inline_clause_referrers(
    index: &ProjectIndex,
    inline_re: &Regex,
    target_id: &str,
) -> Vec<String> {
    let mut referrers = Vec::new();

    for rfc in &index.rfcs {
        for clause in &rfc.clauses {
            let source_id = format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id);
            if source_id != target_id && text_references(inline_re, &clause.spec.text, target_id) {
                referrers.push(source_id);
            }
        }
        if rfc.rfc.changelog.iter().any(|entry| {
            entry
                .notes
                .as_deref()
                .is_some_and(|text| text_references(inline_re, text, target_id))
                || [
                    &entry.added,
                    &entry.changed,
                    &entry.deprecated,
                    &entry.removed,
                    &entry.fixed,
                    &entry.security,
                ]
                .into_iter()
                .flatten()
                .any(|text| text_references(inline_re, text, target_id))
        }) {
            referrers.push(rfc.rfc.rfc_id.clone());
        }
    }

    for adr in &index.adrs {
        let content = &adr.spec.content;
        let direct_content = [&content.context, &content.decision, &content.consequences];
        let alternative_content = content.alternatives.iter().flat_map(|alternative| {
            std::iter::once(&alternative.text)
                .chain(alternative.pros.iter())
                .chain(alternative.cons.iter())
                .chain(alternative.rejection_reason.iter())
        });
        if direct_content
            .into_iter()
            .chain(alternative_content)
            .any(|text| text_references(inline_re, text, target_id))
        {
            referrers.push(adr.spec.govctl.id.clone());
        }
    }

    for work in &index.work_items {
        let content = &work.spec.content;
        let mut direct_content = std::iter::once(&content.description)
            .chain(content.acceptance_criteria.iter().map(|item| &item.text))
            .chain(content.notes.iter())
            .chain(content.journal.iter().map(|entry| &entry.content));
        if direct_content.any(|text| text_references(inline_re, text, target_id)) {
            referrers.push(work.spec.govctl.id.clone());
        }
    }

    referrers
}

fn text_references(inline_re: &Regex, text: &str, target_id: &str) -> bool {
    inline_re
        .captures_iter(text)
        .filter_map(|captures| captures.get(1))
        .any(|target| target.as_str() == target_id)
}

pub(super) fn work_item_deletion_referrers(index: &ProjectIndex, work_id: &str) -> Vec<String> {
    project_referrers(index, work_id, Some(work_id), true)
}

fn project_referrers(
    index: &ProjectIndex,
    target_id: &str,
    skip_work_id: Option<&str>,
    include_work_dependencies: bool,
) -> Vec<String> {
    let mut referenced_by = Vec::new();

    for rfc in &index.rfcs {
        if rfc.rfc.refs.iter().any(|ref_id| ref_id == target_id) {
            referenced_by.push(rfc.rfc.rfc_id.clone());
        }
    }

    for adr in &index.adrs {
        if adr
            .spec
            .govctl
            .refs
            .iter()
            .any(|ref_id| ref_id == target_id)
        {
            referenced_by.push(adr.spec.govctl.id.clone());
        }
    }

    for work in &index.work_items {
        if skip_work_id == Some(work.spec.govctl.id.as_str()) {
            continue;
        }

        let has_ref = work
            .spec
            .govctl
            .refs
            .iter()
            .any(|ref_id| ref_id == target_id);
        let has_dependency = include_work_dependencies
            && work
                .spec
                .govctl
                .depends_on
                .iter()
                .any(|dep_id| dep_id == target_id);

        if has_ref || has_dependency {
            referenced_by.push(work.spec.govctl.id.clone());
        }
    }

    referenced_by
}

fn guard_referrers(config: &Config, target_id: &str) -> DiagnosticResult<Vec<String>> {
    Ok(crate::parse::load_guards(config)?
        .into_iter()
        .filter(|guard| guard.meta().refs.iter().any(|ref_id| ref_id == target_id))
        .map(|guard| guard.meta().id.clone())
        .collect())
}
