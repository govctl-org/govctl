//! Schema validation and state machine rules.
//!
//! Implements validation per [[RFC-0000]] and [[RFC-0001]]:
//! - [[ADR-0003]] signature verification for rendered projections
//! - [[ADR-0010]] placeholder description detection
//! - [[RFC-0000:C-REFERENCE-HIERARCHY]] structured refs and [[...]] link targets

use crate::cmd::edit::rules as edit_rules;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::find_clause_json;
use crate::model::{
    AdrStatus, ClauseStatus, ProjectIndex, ReleasesFile, RfcIndex, RfcPhase, RfcStatus,
    WorkItemStatus,
};
use crate::signature::{compute_rfc_signature, extract_signature};
use crate::write::read_clause;
use regex::Regex;
use std::collections::HashSet;

// =============================================================================
// Field Validation System
// =============================================================================

/// Context for field validation
pub struct ValidationContext<'a> {
    pub config: &'a Config,
    /// The artifact being modified (e.g., "RFC-0001:C-NAME")
    pub artifact_id: &'a str,
}

/// Artifact kinds for validation dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Adr and WorkItem will be used as validation expands
pub enum ArtifactKind {
    Rfc,
    Clause,
    Adr,
    WorkItem,
}

impl ArtifactKind {
    fn as_ssot_artifact(self) -> &'static str {
        match self {
            Self::Rfc => "rfc",
            Self::Clause => "clause",
            Self::Adr => "adr",
            Self::WorkItem => "work",
        }
    }
}

/// Field validation rules
#[derive(Debug, Clone)]
pub enum FieldValidation {
    /// No validation required
    None,
    /// Must be valid semver (e.g., "1.2.3")
    Semver,
    /// Must be a valid clause reference within same RFC, target must be active
    ClauseSupersededBy,
    /// Must be a valid artifact reference (RFC-xxx, ADR-xxx, etc.)
    ArtifactRef,
    /// Must be a valid enum value (validated by serde)
    EnumValue,
}

impl FieldValidation {
    /// Get the validation rule for a field
    pub fn for_field(kind: ArtifactKind, field: &str) -> Self {
        match edit_rules::field_validation_kind(kind.as_ssot_artifact(), field) {
            Some(edit_rules::ValidationKind::Semver) => Self::Semver,
            Some(edit_rules::ValidationKind::ClauseSupersededBy) => Self::ClauseSupersededBy,
            Some(edit_rules::ValidationKind::ArtifactRef) => Self::ArtifactRef,
            Some(edit_rules::ValidationKind::EnumValue) => Self::EnumValue,
            None => Self::None,
        }
    }

    /// Validate a value
    pub fn validate(&self, ctx: &ValidationContext, value: &str) -> anyhow::Result<()> {
        match self {
            Self::None => Ok(()),
            Self::EnumValue => Ok(()), // Validated by serde during parse
            Self::Semver => validate_semver(value),
            Self::ClauseSupersededBy => validate_clause_superseded_by(ctx, value),
            Self::ArtifactRef => validate_artifact_ref(ctx, value),
        }
    }
}

/// Validate a semver string
fn validate_semver(value: &str) -> anyhow::Result<()> {
    semver::Version::parse(value).map_err(|_| {
        Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            format!("Invalid semver: {value}"),
            value,
        )
    })?;
    Ok(())
}

/// Validate a clause superseded_by reference
fn validate_clause_superseded_by(ctx: &ValidationContext, target: &str) -> anyhow::Result<()> {
    // Empty string means "clear the field"
    if target.is_empty() {
        return Ok(());
    }

    // Extract RFC ID from source clause (e.g., "RFC-0001:C-NAME" -> "RFC-0001")
    let (source_rfc, source_clause) = ctx.artifact_id.split_once(':').ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            format!("Invalid clause ID format: {}", ctx.artifact_id),
            ctx.artifact_id,
        )
    })?;
    if source_rfc.is_empty() || source_clause.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            format!("Invalid clause ID format: {}", ctx.artifact_id),
            ctx.artifact_id,
        )
        .into());
    }

    // Build full target reference
    let full_target = if target.contains(':') {
        target.to_string()
    } else {
        format!("{source_rfc}:{target}")
    };

    // Check target is in same RFC
    let (target_rfc, target_clause) = full_target.split_once(':').ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            format!("Invalid target clause ID: {target}"),
            target,
        )
    })?;
    if target_rfc.is_empty() || target_clause.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            format!("Invalid target clause ID: {target}"),
            target,
        )
        .into());
    }

    if target_rfc != source_rfc {
        return Err(Diagnostic::new(
            DiagnosticCode::E0206ClauseSupersededByUnknown,
            format!(
                "superseded_by must reference a clause in the same RFC (got {target_rfc}, expected {source_rfc})"
            ),
            target,
        )
        .into());
    }

    // Check target clause exists
    let target_path = find_clause_json(ctx.config, &full_target).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Target clause not found: {full_target}"),
            &full_target,
        )
    })?;

    // Check target clause is active (not superseded or deprecated)
    let target_clause = read_clause(ctx.config, &target_path)?;
    match target_clause.status {
        ClauseStatus::Active => Ok(()),
        ClauseStatus::Superseded => Err(Diagnostic::new(
            DiagnosticCode::E0207ClauseSupersededByNotActive,
            format!("Cannot supersede by a superseded clause: {full_target}"),
            &full_target,
        )
        .into()),
        ClauseStatus::Deprecated => Err(Diagnostic::new(
            DiagnosticCode::E0207ClauseSupersededByNotActive,
            format!("Cannot supersede by a deprecated clause: {full_target}"),
            &full_target,
        )
        .into()),
    }
}

/// Validate an artifact reference exists and respects [[RFC-0000:C-REFERENCE-HIERARCHY]].
fn validate_artifact_ref(ctx: &ValidationContext, ref_id: &str) -> anyhow::Result<()> {
    use crate::load::find_rfc_json;
    use crate::parse::{load_adrs, load_work_items};

    if ref_id.starts_with("RFC-") {
        if find_rfc_json(ctx.config, ref_id).is_none() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else if ref_id.starts_with("ADR-") {
        let adrs = load_adrs(ctx.config)?;
        if !adrs.iter().any(|a| a.spec.govctl.id == ref_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else if ref_id.starts_with("WI-") {
        let items = load_work_items(ctx.config)?;
        if !items.iter().any(|w| w.spec.govctl.id == ref_id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {ref_id}"),
                ref_id,
            )
            .into());
        }
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0813SupersedeNotSupported,
            format!("Unknown artifact type: {ref_id}"),
            ref_id,
        )
        .into());
    }

    check_ref_hierarchy(ctx.artifact_id, ref_id, ctx.artifact_id).map_err(|e| e.into())
}

/// Enforce [[RFC-0000:C-REFERENCE-HIERARCHY]] for `refs` targets.
fn check_ref_hierarchy(
    artifact_id: &str,
    ref_id: &str,
    diagnostic_path: &str,
) -> Result<(), Diagnostic> {
    let owner_is_rfc = artifact_id.starts_with("RFC-");
    let owner_is_adr = artifact_id.starts_with("ADR-");
    let owner_is_wi = artifact_id.starts_with("WI-");

    if owner_is_wi {
        return Ok(());
    }
    if owner_is_rfc && (ref_id.starts_with("ADR-") || ref_id.starts_with("WI-")) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0112RfcReferenceHierarchy,
            format!(
                "RFC '{artifact_id}' references {ref_id}, but RFCs are higher authority than ADRs and Work Items — remove this reference (the ADR or Work Item should reference the RFC, not the other way around)"
            ),
            diagnostic_path,
        ));
    }
    if owner_is_adr && ref_id.starts_with("WI-") {
        return Err(Diagnostic::new(
            DiagnosticCode::E0306AdrReferenceHierarchy,
            format!(
                "ADR '{artifact_id}' references {ref_id}, but ADRs are higher authority than Work Items — remove this reference (the Work Item should reference the ADR, not the other way around)"
            ),
            diagnostic_path,
        ));
    }
    Ok(())
}

// =============================================================================
// Convenience function for commands
// =============================================================================

/// Validate a field value before setting
pub fn validate_field(
    config: &Config,
    artifact_id: &str,
    kind: ArtifactKind,
    field: &str,
    value: &str,
) -> anyhow::Result<()> {
    let ctx = ValidationContext {
        config,
        artifact_id,
    };
    let validation = FieldValidation::for_field(kind, field);
    validation.validate(&ctx, value)
}

/// Validation result with diagnostics
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    pub rfc_count: usize,
    pub clause_count: usize,
    pub adr_count: usize,
    pub work_count: usize,
}

impl ValidationResult {
    #[allow(dead_code)]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == crate::diagnostic::DiagnosticLevel::Error)
    }
}

/// Validate the entire project
pub fn validate_project(index: &ProjectIndex, config: &Config) -> ValidationResult {
    let mut result = ValidationResult {
        rfc_count: index.rfcs.len(),
        clause_count: index.iter_clauses().count(),
        adr_count: index.adrs.len(),
        work_count: index.work_items.len(),
        ..Default::default()
    };

    // Validate RFCs
    for rfc in &index.rfcs {
        validate_rfc(rfc, config, &mut result);
    }

    // Validate RFC signatures (per ADR-0003)
    validate_rfc_signatures(index, config, &mut result);

    // Validate cross-references
    validate_clause_references(index, config, &mut result);

    // Validate ADRs
    for adr in &index.adrs {
        let adr_path_display = config.display_path(&adr.path).display().to_string();
        if adr.meta().refs.is_empty() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0103AdrNoRefs,
                format!(
                    "ADR has no artifact references (hint: `govctl adr add {} refs RFC-XXXX`)",
                    adr.meta().id
                ),
                adr_path_display.clone(),
            ));
        }

        // Validate content is not placeholder
        if adr.spec.content.context.contains("Describe the context") {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0103AdrNoRefs,
                format!(
                    "ADR has placeholder context (hint: `govctl adr set {} context \"...\"`)",
                    adr.meta().id
                ),
                adr_path_display,
            ));
        }
    }

    // Validate artifact references (refs fields)
    validate_artifact_refs(index, config, &mut result);

    // [[...]] in RFC/ADR governed text — [[RFC-0000:C-REFERENCE-HIERARCHY]]
    validate_bracket_reference_hierarchy(index, config, &mut result);

    // Validate work item descriptions
    validate_work_item_descriptions(index, config, &mut result);

    // Validate tags against allowed set — [[RFC-0002:C-RESOURCES]]
    validate_artifact_tags(index, config, &mut result);

    result
}

/// Validate RFC rendered markdown signatures (per ADR-0003)
fn validate_rfc_signatures(index: &ProjectIndex, config: &Config, result: &mut ValidationResult) {
    let output_dir = config.rfc_output();

    for rfc in &index.rfcs {
        let md_path = output_dir.join(format!("{}.md", rfc.rfc.rfc_id));

        // Skip if rendered file doesn't exist yet
        if !md_path.exists() {
            continue;
        }

        // Read rendered markdown
        let md_content = match std::fs::read_to_string(&md_path) {
            Ok(content) => content,
            Err(e) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::W0106RenderedReadError,
                    format!(
                        "Could not read rendered markdown: {} (hint: run `govctl rfc render`)",
                        e
                    ),
                    config.display_path(&md_path).display().to_string(),
                ));
                continue;
            }
        };

        let md_path_display = config.display_path(&md_path).display().to_string();

        // Extract signature from rendered markdown
        let Some(existing_sig) = extract_signature(&md_content) else {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0602SignatureMissing,
                format!(
                    "Rendered markdown missing signature. Run 'govctl render' to regenerate: {}",
                    rfc.rfc.rfc_id
                ),
                md_path_display.clone(),
            ));
            continue;
        };

        // Compute expected signature from source
        let expected_sig = match compute_rfc_signature(rfc) {
            Ok(sig) => sig,
            Err(e) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0601SignatureMismatch,
                    format!("Failed to compute signature for {}: {}", rfc.rfc.rfc_id, e),
                    md_path_display.clone(),
                ));
                continue;
            }
        };

        // Compare
        if existing_sig != expected_sig {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0601SignatureMismatch,
                format!(
                    "Signature mismatch: rendered markdown was edited directly or source changed. Run 'govctl render' to regenerate: {}",
                    rfc.rfc.rfc_id
                ),
                md_path_display,
            ));
        }
    }
}

/// Validate a single RFC
fn validate_rfc(rfc: &RfcIndex, config: &Config, result: &mut ValidationResult) {
    let rfc_path_display = config.display_path(&rfc.path).display().to_string();

    // Check RFC ID matches directory
    let dir_name = rfc
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    if let Some(name) = dir_name
        && name != rfc.rfc.rfc_id
    {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0103RfcIdMismatch,
            format!(
                "RFC ID '{}' doesn't match directory '{}'",
                rfc.rfc.rfc_id, name
            ),
            rfc_path_display.clone(),
        ));
    }

    // Check changelog exists
    if rfc.rfc.changelog.is_empty() {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::W0101RfcNoChangelog,
            "RFC has no changelog entries (hint: run `govctl rfc bump`)",
            rfc_path_display.clone(),
        ));
    }

    // Validate status/phase constraints
    validate_status_phase_constraints(rfc, config, result);

    // Validate clauses
    for clause in &rfc.clauses {
        let clause_path_display = config.display_path(&clause.path).display().to_string();
        // Check clause has 'since' field
        if clause.spec.since.is_none() {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0102ClauseNoSince,
                format!(
                    "Clause '{}' has no 'since' version (hint: it will be set automatically by `govctl rfc bump` or `govctl rfc finalize`)",
                    clause.spec.clause_id
                ),
                clause_path_display.clone(),
            ));
        }

        // Check clause ID matches filename
        let file_name = clause
            .path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name != clause.spec.clause_id {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0203ClauseIdMismatch,
                format!(
                    "Clause ID '{}' doesn't match filename '{}'",
                    clause.spec.clause_id, file_name
                ),
                clause_path_display,
            ));
        }
    }
}

/// Validate status/phase constraints per RFC-0000
fn validate_status_phase_constraints(
    rfc: &RfcIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let status = rfc.rfc.status;
    let phase = rfc.rfc.phase;
    let path_display = config.display_path(&rfc.path).display().to_string();

    // draft + stable is forbidden
    if status == RfcStatus::Draft && phase == RfcPhase::Stable {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            "Cannot have status=draft with phase=stable",
            path_display.clone(),
        ));
    }

    // deprecated + impl/test is forbidden
    if status == RfcStatus::Deprecated && (phase == RfcPhase::Impl || phase == RfcPhase::Test) {
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Cannot have status=deprecated with phase={}",
                phase.as_ref()
            ),
            path_display,
        ));
    }
}

/// Validate clause cross-references (superseded_by)
fn validate_clause_references(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    // Collect all active clause IDs
    let active_clauses: HashSet<String> = index
        .iter_clauses()
        .filter(|(_, c)| c.spec.status == ClauseStatus::Active)
        .map(|(rfc, c)| format!("{}:{}", rfc.rfc.rfc_id, c.spec.clause_id))
        .collect();

    // Check superseded_by references
    for (rfc, clause) in index.iter_clauses() {
        if let Some(ref superseded_by) = clause.spec.superseded_by {
            let clause_path_display = config.display_path(&clause.path).display().to_string();
            // If superseded, status should be Superseded
            if clause.spec.status != ClauseStatus::Superseded {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0206ClauseSupersededByUnknown,
                    format!(
                        "Clause has superseded_by but status is not 'superseded': {}",
                        clause.spec.clause_id
                    ),
                    clause_path_display.clone(),
                ));
            }

            // Build full reference
            let full_ref = if superseded_by.contains(':') {
                superseded_by.clone()
            } else {
                format!("{}:{}", rfc.rfc.rfc_id, superseded_by)
            };

            // Check reference exists and is active
            if !active_clauses.contains(&full_ref) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0207ClauseSupersededByNotActive,
                    format!(
                        "Clause '{}' superseded by '{}' which is not active",
                        clause.spec.clause_id, superseded_by
                    ),
                    clause_path_display,
                ));
            }
        }
    }
}

/// Validate `[[...]]` link targets in RFC and ADR content per [[RFC-0000:C-REFERENCE-HIERARCHY]].
fn validate_bracket_reference_hierarchy(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let re = match Regex::new(&config.source_scan.pattern) {
        Ok(r) => r,
        Err(e) => {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0501ConfigInvalid,
                format!("Invalid source_scan.pattern for bracket reference scan: {e}"),
                "gov/config.toml".to_string(),
            ));
            return;
        }
    };

    for rfc in &index.rfcs {
        let rfc_path = config.display_path(&rfc.path).display().to_string();
        let rid = rfc.rfc.rfc_id.as_str();
        for clause in &rfc.clauses {
            let clause_path = config.display_path(&clause.path).display().to_string();
            scan_rfc_bracket_refs(&re, &clause.spec.text, rid, &clause_path, result);
        }
        for entry in &rfc.rfc.changelog {
            if let Some(ref notes) = entry.notes {
                scan_rfc_bracket_refs(&re, notes, rid, &rfc_path, result);
            }
            for line in entry
                .added
                .iter()
                .chain(entry.changed.iter())
                .chain(entry.deprecated.iter())
                .chain(entry.removed.iter())
                .chain(entry.fixed.iter())
                .chain(entry.security.iter())
            {
                scan_rfc_bracket_refs(&re, line, rid, &rfc_path, result);
            }
        }
    }

    for adr in &index.adrs {
        let adr_path = config.display_path(&adr.path).display().to_string();
        let aid = adr.meta().id.as_str();
        let c = &adr.spec.content;
        scan_adr_bracket_refs(&re, &c.context, aid, &adr_path, result);
        scan_adr_bracket_refs(&re, &c.decision, aid, &adr_path, result);
        scan_adr_bracket_refs(&re, &c.consequences, aid, &adr_path, result);
        for alt in &c.alternatives {
            scan_adr_bracket_refs(&re, &alt.text, aid, &adr_path, result);
            for p in &alt.pros {
                scan_adr_bracket_refs(&re, p, aid, &adr_path, result);
            }
            for cons in &alt.cons {
                scan_adr_bracket_refs(&re, cons, aid, &adr_path, result);
            }
            if let Some(ref rr) = alt.rejection_reason {
                scan_adr_bracket_refs(&re, rr, aid, &adr_path, result);
            }
        }
    }
}

fn scan_rfc_bracket_refs(
    re: &Regex,
    text: &str,
    rfc_id: &str,
    path: &str,
    result: &mut ValidationResult,
) {
    for caps in re.captures_iter(text) {
        let Some(m) = caps.get(1) else {
            continue;
        };
        let target = m.as_str();
        if target.starts_with("ADR-") || target.starts_with("WI-") {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0112RfcReferenceHierarchy,
                format!(
                    "RFC '{rfc_id}' links to [[{target}]], but RFCs are higher authority than ADRs and Work Items — remove this link (the ADR or Work Item should reference the RFC, not the other way around)"
                ),
                path,
            ));
        }
    }
}

fn scan_adr_bracket_refs(
    re: &Regex,
    text: &str,
    adr_id: &str,
    path: &str,
    result: &mut ValidationResult,
) {
    for caps in re.captures_iter(text) {
        let Some(m) = caps.get(1) else {
            continue;
        };
        let target = m.as_str();
        if target.starts_with("WI-") {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0306AdrReferenceHierarchy,
                format!("ADR '{adr_id}' links to [[{target}]], but ADRs are higher authority than Work Items — remove this link (the Work Item should reference the ADR, not the other way around)"),
                path,
            ));
        }
    }
}

/// Validate refs fields in RFCs, ADRs and Work Items
fn validate_artifact_refs(index: &ProjectIndex, config: &Config, result: &mut ValidationResult) {
    // Build a set of all known artifact IDs (including clause references)
    let mut known_ids: HashSet<String> = HashSet::new();

    // Add RFC IDs and clause references
    for rfc in &index.rfcs {
        known_ids.insert(rfc.rfc.rfc_id.clone());
        // Add clause references in format RFC-ID:CLAUSE-ID
        for clause in &rfc.clauses {
            known_ids.insert(format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id));
        }
    }

    // Add ADR IDs
    for adr in &index.adrs {
        known_ids.insert(adr.meta().id.clone());
    }

    // Add Work Item IDs
    for work in &index.work_items {
        known_ids.insert(work.meta().id.clone());
    }

    // Validate RFC refs and supersedes
    for rfc in &index.rfcs {
        let rfc_path_display = config.display_path(&rfc.path).display().to_string();
        // Validate refs field
        for ref_id in &rfc.rfc.refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0105RfcRefNotFound,
                    format!(
                        "RFC '{}' references unknown artifact: {}",
                        rfc.rfc.rfc_id, ref_id
                    ),
                    rfc_path_display.clone(),
                ));
            } else if let Err(d) = check_ref_hierarchy(&rfc.rfc.rfc_id, ref_id, &rfc_path_display) {
                result.diagnostics.push(d);
            }
        }

        // Validate supersedes field
        if let Some(ref supersedes) = rfc.rfc.supersedes {
            if !known_ids.contains(supersedes) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0106RfcSupersedesNotFound,
                    format!(
                        "RFC '{}' supersedes unknown RFC: {}",
                        rfc.rfc.rfc_id, supersedes
                    ),
                    rfc_path_display.clone(),
                ));
            } else if let Err(d) =
                check_ref_hierarchy(&rfc.rfc.rfc_id, supersedes, &rfc_path_display)
            {
                result.diagnostics.push(d);
            }
        }
    }

    // Validate ADR refs
    for adr in &index.adrs {
        let adr_path_display = config.display_path(&adr.path).display().to_string();
        for ref_id in &adr.meta().refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0304AdrRefNotFound,
                    format!(
                        "ADR '{}' references unknown artifact: {}",
                        adr.meta().id,
                        ref_id
                    ),
                    adr_path_display.clone(),
                ));
            } else if let Err(d) = check_ref_hierarchy(&adr.meta().id, ref_id, &adr_path_display) {
                result.diagnostics.push(d);
            }
        }
    }

    // Validate Work Item refs
    for work in &index.work_items {
        let work_path_display = config.display_path(&work.path).display().to_string();
        for ref_id in &work.meta().refs {
            if !known_ids.contains(ref_id) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0404WorkRefNotFound,
                    format!(
                        "Work item '{}' references unknown artifact: {}",
                        work.meta().id,
                        ref_id
                    ),
                    work_path_display.clone(),
                ));
            }
        }
    }
}

/// Validate release metadata and work item references.
pub fn validate_releases(
    releases: &ReleasesFile,
    index: &ProjectIndex,
    config: &Config,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_versions = HashSet::new();
    let known_work_ids: HashSet<&str> = index
        .work_items
        .iter()
        .map(|work| work.meta().id.as_str())
        .collect();
    let releases_display = config
        .display_path(&config.releases_path())
        .display()
        .to_string();

    for release in &releases.releases {
        if !seen_versions.insert(release.version.as_str()) {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0702ReleaseDuplicate,
                format!("Duplicate release version: {}", release.version),
                releases_display.clone(),
            ));
        }

        for work_id in &release.refs {
            if !known_work_ids.contains(work_id.as_str()) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0705ReleaseRefNotFound,
                    format!(
                        "Release '{}' references unknown work item: {}",
                        release.version, work_id
                    ),
                    releases_display.clone(),
                ));
            }
        }
    }

    diagnostics
}

/// Check if a work item description is a placeholder or empty
fn is_placeholder_description(desc: &str) -> bool {
    let trimmed = desc.trim();

    // Empty or whitespace-only
    if trimmed.is_empty() {
        return true;
    }

    // Exact template match
    if trimmed.contains("Describe the work to be done")
        && trimmed.contains("What is the goal?")
        && trimmed.contains("What are the acceptance criteria?")
    {
        return true;
    }

    // Common placeholder patterns (case-insensitive)
    let lower = trimmed.to_lowercase();
    let placeholder_patterns = ["todo", "tbd", "fill in later", "placeholder", "fixme"];

    // Only flag if the entire description is just a placeholder word
    placeholder_patterns
        .iter()
        .any(|p| lower == *p || lower == format!("[{}]", p) || lower == format!("<{}>", p))
}

/// Validate work item descriptions for placeholder content (per ADR-0010)
fn validate_work_item_descriptions(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    for work in &index.work_items {
        let desc = &work.spec.content.description;
        if is_placeholder_description(desc) {
            let path_display = config.display_path(&work.path).display().to_string();
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0108WorkPlaceholderDescription,
                format!(
                    "Work item has placeholder description (hint: `govctl work set {} description \"...\"`)",
                    work.meta().id
                ),
                path_display,
            ));
        }
    }
}

/// Validate that all artifact tags are in the allowed set and well-formed.
///
/// Per [[RFC-0002:C-RESOURCES]] controlled-vocabulary tags: every tag used by an
/// artifact must be listed in config.toml [tags] allowed, and each tag must match
/// the format `^[a-z][a-z0-9-]*$`.
fn validate_artifact_tags(index: &ProjectIndex, config: &Config, result: &mut ValidationResult) {
    let allowed = &config.tags.allowed;

    let mut check_tags = |tags: &[String], artifact_id: &str, path_display: &str| {
        for tag in tags {
            // Validate format
            let tag_re = match crate::cmd::tag::tag_re() {
                Ok(r) => r,
                Err(e) => {
                    result.diagnostics.push(Diagnostic::new(
                        DiagnosticCode::E0806InvalidPattern,
                        format!("Failed to compile tag regex: {e}"),
                        path_display,
                    ));
                    continue;
                }
            };
            if !tag_re.is_match(tag) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E1101TagInvalidFormat,
                    format!(
                        "Artifact '{artifact_id}' has invalid tag format '{tag}': must match ^[a-z][a-z0-9-]*$"
                    ),
                    path_display,
                ));
                continue;
            }
            // Validate against allowed set (deny-all when empty)
            if !allowed.contains(tag) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E1105TagUnknown,
                    format!(
                        "Artifact '{artifact_id}' uses unknown tag '{tag}' (not in config.toml [tags] allowed)"
                    ),
                    path_display,
                ));
            }
        }
    };

    for rfc in &index.rfcs {
        let path = config.display_path(&rfc.path).display().to_string();
        check_tags(&rfc.rfc.tags, &rfc.rfc.rfc_id, &path);
    }

    for (rfc, clause) in index.iter_clauses() {
        let clause_id = format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id);
        let path = config.display_path(&clause.path).display().to_string();
        check_tags(&clause.spec.tags, &clause_id, &path);
    }

    for adr in &index.adrs {
        let path = config.display_path(&adr.path).display().to_string();
        check_tags(&adr.spec.govctl.tags, &adr.meta().id, &path);
    }

    for work in &index.work_items {
        let path = config.display_path(&work.path).display().to_string();
        check_tags(&work.spec.govctl.tags, &work.meta().id, &path);
    }

    if let Ok(guard_result) = crate::parse::load_guards_with_warnings(config) {
        for guard in &guard_result.items {
            let path = config.display_path(&guard.path).display().to_string();
            check_tags(&guard.spec.govctl.tags, &guard.meta().id, &path);
        }
    }
}

/// Check if RFC status transition is valid
pub fn is_valid_status_transition(from: RfcStatus, to: RfcStatus) -> bool {
    matches!(
        (from, to),
        (RfcStatus::Draft, RfcStatus::Normative) | (RfcStatus::Normative, RfcStatus::Deprecated)
    )
}

/// Check if RFC phase transition is valid
pub fn is_valid_phase_transition(from: RfcPhase, to: RfcPhase) -> bool {
    matches!(
        (from, to),
        (RfcPhase::Spec, RfcPhase::Impl)
            | (RfcPhase::Impl, RfcPhase::Test)
            | (RfcPhase::Test, RfcPhase::Stable)
    )
}

/// Check if ADR status transition is valid
/// ADR lifecycle: proposed → accepted → superseded
///                        → rejected
pub fn is_valid_adr_transition(from: AdrStatus, to: AdrStatus) -> bool {
    matches!(
        (from, to),
        (AdrStatus::Proposed, AdrStatus::Accepted)
            | (AdrStatus::Proposed, AdrStatus::Rejected)
            | (AdrStatus::Accepted, AdrStatus::Superseded)
    )
}

/// Check if Work Item status transition is valid
pub fn is_valid_work_transition(from: WorkItemStatus, to: WorkItemStatus) -> bool {
    matches!(
        (from, to),
        (WorkItemStatus::Queue, WorkItemStatus::Active)
            | (WorkItemStatus::Active, WorkItemStatus::Done)
            | (WorkItemStatus::Queue, WorkItemStatus::Cancelled)
            | (WorkItemStatus::Active, WorkItemStatus::Cancelled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // RFC Status Transition Tests
    // =========================================================================

    #[test]
    fn test_rfc_status_draft_to_normative() {
        assert!(is_valid_status_transition(
            RfcStatus::Draft,
            RfcStatus::Normative
        ));
    }

    #[test]
    fn test_rfc_status_normative_to_deprecated() {
        assert!(is_valid_status_transition(
            RfcStatus::Normative,
            RfcStatus::Deprecated
        ));
    }

    #[test]
    fn test_rfc_status_invalid_draft_to_deprecated() {
        assert!(!is_valid_status_transition(
            RfcStatus::Draft,
            RfcStatus::Deprecated
        ));
    }

    #[test]
    fn test_rfc_status_invalid_normative_to_draft() {
        assert!(!is_valid_status_transition(
            RfcStatus::Normative,
            RfcStatus::Draft
        ));
    }

    #[test]
    fn test_rfc_status_invalid_deprecated_to_normative() {
        assert!(!is_valid_status_transition(
            RfcStatus::Deprecated,
            RfcStatus::Normative
        ));
    }

    #[test]
    fn test_rfc_status_same_state() {
        assert!(!is_valid_status_transition(
            RfcStatus::Draft,
            RfcStatus::Draft
        ));
        assert!(!is_valid_status_transition(
            RfcStatus::Normative,
            RfcStatus::Normative
        ));
    }

    // =========================================================================
    // RFC Phase Transition Tests
    // =========================================================================

    #[test]
    fn test_rfc_phase_spec_to_impl() {
        assert!(is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Impl));
    }

    #[test]
    fn test_rfc_phase_impl_to_test() {
        assert!(is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Test));
    }

    #[test]
    fn test_rfc_phase_test_to_stable() {
        assert!(is_valid_phase_transition(RfcPhase::Test, RfcPhase::Stable));
    }

    #[test]
    fn test_rfc_phase_invalid_skip() {
        // Cannot skip phases
        assert!(!is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Test));
        assert!(!is_valid_phase_transition(RfcPhase::Spec, RfcPhase::Stable));
        assert!(!is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Stable));
    }

    #[test]
    fn test_rfc_phase_invalid_backward() {
        assert!(!is_valid_phase_transition(RfcPhase::Stable, RfcPhase::Test));
        assert!(!is_valid_phase_transition(RfcPhase::Test, RfcPhase::Impl));
        assert!(!is_valid_phase_transition(RfcPhase::Impl, RfcPhase::Spec));
    }

    // =========================================================================
    // ADR Status Transition Tests
    // =========================================================================

    #[test]
    fn test_adr_status_proposed_to_accepted() {
        assert!(is_valid_adr_transition(
            AdrStatus::Proposed,
            AdrStatus::Accepted
        ));
    }

    #[test]
    fn test_adr_status_accepted_to_superseded() {
        assert!(is_valid_adr_transition(
            AdrStatus::Accepted,
            AdrStatus::Superseded
        ));
    }

    #[test]
    fn test_adr_status_proposed_to_rejected() {
        assert!(is_valid_adr_transition(
            AdrStatus::Proposed,
            AdrStatus::Rejected
        ));
    }

    #[test]
    fn test_adr_status_invalid_proposed_to_superseded() {
        assert!(!is_valid_adr_transition(
            AdrStatus::Proposed,
            AdrStatus::Superseded
        ));
    }

    #[test]
    fn test_adr_status_invalid_rejected_transitions() {
        // Rejected is terminal
        assert!(!is_valid_adr_transition(
            AdrStatus::Rejected,
            AdrStatus::Accepted
        ));
        assert!(!is_valid_adr_transition(
            AdrStatus::Rejected,
            AdrStatus::Proposed
        ));
    }

    #[test]
    fn test_adr_status_invalid_backward() {
        assert!(!is_valid_adr_transition(
            AdrStatus::Accepted,
            AdrStatus::Proposed
        ));
        assert!(!is_valid_adr_transition(
            AdrStatus::Superseded,
            AdrStatus::Accepted
        ));
    }

    // =========================================================================
    // Work Item Status Transition Tests
    // =========================================================================

    #[test]
    fn test_work_status_queue_to_active() {
        assert!(is_valid_work_transition(
            WorkItemStatus::Queue,
            WorkItemStatus::Active
        ));
    }

    #[test]
    fn test_work_status_active_to_done() {
        assert!(is_valid_work_transition(
            WorkItemStatus::Active,
            WorkItemStatus::Done
        ));
    }

    #[test]
    fn test_work_status_queue_to_cancelled() {
        assert!(is_valid_work_transition(
            WorkItemStatus::Queue,
            WorkItemStatus::Cancelled
        ));
    }

    #[test]
    fn test_work_status_active_to_cancelled() {
        assert!(is_valid_work_transition(
            WorkItemStatus::Active,
            WorkItemStatus::Cancelled
        ));
    }

    #[test]
    fn test_work_status_invalid_queue_to_done() {
        // Cannot skip active
        assert!(!is_valid_work_transition(
            WorkItemStatus::Queue,
            WorkItemStatus::Done
        ));
    }

    #[test]
    fn test_work_status_invalid_done_transitions() {
        // Done is terminal (except requeue which isn't implemented)
        assert!(!is_valid_work_transition(
            WorkItemStatus::Done,
            WorkItemStatus::Active
        ));
        assert!(!is_valid_work_transition(
            WorkItemStatus::Done,
            WorkItemStatus::Queue
        ));
    }

    #[test]
    fn test_work_status_invalid_cancelled_transitions() {
        // Cancelled is terminal
        assert!(!is_valid_work_transition(
            WorkItemStatus::Cancelled,
            WorkItemStatus::Active
        ));
        assert!(!is_valid_work_transition(
            WorkItemStatus::Cancelled,
            WorkItemStatus::Queue
        ));
    }

    // =========================================================================
    // FieldValidation::for_field Tests
    // =========================================================================

    #[test]
    fn test_field_validation_clause_since() {
        assert!(matches!(
            FieldValidation::for_field(ArtifactKind::Clause, "since"),
            FieldValidation::Semver
        ));
    }

    #[test]
    fn test_field_validation_clause_superseded_by() {
        assert!(matches!(
            FieldValidation::for_field(ArtifactKind::Clause, "superseded_by"),
            FieldValidation::ClauseSupersededBy
        ));
    }

    #[test]
    fn test_field_validation_rfc_version() {
        assert!(matches!(
            FieldValidation::for_field(ArtifactKind::Rfc, "version"),
            FieldValidation::Semver
        ));
    }

    #[test]
    fn test_field_validation_unknown_field() {
        assert!(matches!(
            FieldValidation::for_field(ArtifactKind::Rfc, "unknown"),
            FieldValidation::None
        ));
    }

    // =========================================================================
    // Reference hierarchy ([[RFC-0000:C-REFERENCE-HIERARCHY]])
    // =========================================================================

    #[test]
    fn test_ref_hierarchy_rfc_rejects_adr_and_wi() {
        assert!(check_ref_hierarchy("RFC-0001", "ADR-0001", "f").is_err());
        assert!(check_ref_hierarchy("RFC-0001", "WI-2026-01-01-001", "f").is_err());
    }

    #[test]
    fn test_ref_hierarchy_rfc_allows_rfc_and_clause() {
        assert!(check_ref_hierarchy("RFC-0001", "RFC-0002", "f").is_ok());
        assert!(check_ref_hierarchy("RFC-0001", "RFC-0002:C-FOO", "f").is_ok());
    }

    #[test]
    fn test_ref_hierarchy_adr_rejects_wi() {
        assert!(check_ref_hierarchy("ADR-0001", "WI-2026-01-01-001", "f").is_err());
    }

    #[test]
    fn test_ref_hierarchy_adr_allows_rfc_adr() {
        assert!(check_ref_hierarchy("ADR-0001", "RFC-0000:C-RFC-DEF", "f").is_ok());
        assert!(check_ref_hierarchy("ADR-0001", "ADR-0002", "f").is_ok());
    }

    #[test]
    fn test_ref_hierarchy_work_allows_any() {
        assert!(check_ref_hierarchy("WI-2026-01-01-001", "WI-2026-01-01-002", "f").is_ok());
        assert!(check_ref_hierarchy("WI-2026-01-01-001", "ADR-0001", "f").is_ok());
    }
}
