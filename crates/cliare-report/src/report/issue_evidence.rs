use super::actions::evidence_for_finding;
use super::util::unique_strings;
use super::*;
use crate::report_evidence::{ProcessEvidence, SideEffectRecord};
use cliare_policy::path_classification;

pub(super) fn issue_evidence_references(
    item: &ActionItem,
    artifacts: &MeasuredArtifacts,
) -> Vec<String> {
    if let Some(kind) = item.id.strip_prefix("shape.gap.") {
        return unique_strings(
            artifacts
                .shape
                .gaps
                .iter()
                .filter(|gap| gap.kind == kind)
                .flat_map(|gap| gap.evidence.iter().cloned())
                .collect(),
        );
    }

    if item.id.starts_with("finding.")
        && let Some(finding) = artifacts
            .scorecard
            .findings
            .iter()
            .find(|finding| finding.id == item.id)
    {
        return unique_strings(evidence_for_finding(finding, artifacts));
    }

    item.evidence.clone()
}

pub(super) fn issue_evidence(
    references: &[String],
    evidence: &EvidenceSummary,
    category: ActionCategory,
) -> Vec<IssueEvidence> {
    let mut references = unique_strings(references.to_vec());
    references.sort_by(|left, right| {
        evidence_reference_rank(left, evidence)
            .cmp(&evidence_reference_rank(right, evidence))
            .then(left.cmp(right))
    });

    references
        .iter()
        .take(ACTION_EVIDENCE_LIMIT)
        .map(|reference| {
            let event_id = reference
                .split_once(':')
                .map_or(reference.as_str(), |(id, _)| id);
            if let Some(process) = evidence.processes.get(event_id) {
                let side_effect_records = if category == ActionCategory::Safety {
                    evidence
                        .side_effects
                        .iter()
                        .filter(|record| record.evidence == event_id)
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                let detail = if side_effect_records.is_empty() {
                    process_detail_for_reference(process, reference)
                } else {
                    format!(
                        "{}; {}",
                        process.summary(),
                        side_effect_summary(&side_effect_records)
                    )
                };
                let interpretation = if side_effect_records.is_empty() {
                    None
                } else {
                    Some(
                        "A safe discovery probe changed persistent filesystem state; review whether this write is expected, documented, and allowed by policy."
                            .to_owned(),
                    )
                };
                let side_effects = side_effect_records
                    .iter()
                    .map(|record| IssueSideEffect {
                        operation: record.kind.clone(),
                        region: record.region.clone(),
                        path: record.path.clone(),
                        credential_like: path_classification::credential_like_path_text(&record.path),
                        size_bytes: record.size_bytes,
                    })
                    .collect::<Vec<_>>();
                IssueEvidence {
                    kind: if side_effects.is_empty() {
                        "process".to_owned()
                    } else {
                        "side_effect".to_owned()
                    },
                    reference: reference.clone(),
                    detail,
                    probe_id: Some(process.probe_id.clone()),
                    intent: process.intent.clone(),
                    scope: process.scope_label(),
                    argv: process.argv.clone(),
                    status: Some(process.status.clone()),
                    interpretation,
                    side_effects,
                }
            } else {
                IssueEvidence {
                    kind: "shape".to_owned(),
                    reference: reference.clone(),
                    detail: "shape-derived evidence reference".to_owned(),
                    probe_id: None,
                    intent: None,
                    scope: "shape inference".to_owned(),
                    argv: Vec::new(),
                    status: None,
                    interpretation: None,
                    side_effects: Vec::new(),
                }
            }
        })
        .collect()
}

fn process_detail_for_reference(process: &ProcessEvidence, reference: &str) -> String {
    let suffix = reference
        .split_once(':')
        .map_or("", |(_, suffix)| suffix)
        .to_ascii_lowercase();
    if suffix.contains("precondition") || suffix.contains("blocked") {
        return format!(
            "{}; classified as a runtime precondition",
            process.summary()
        );
    }
    process.summary()
}

fn side_effect_summary(records: &[&SideEffectRecord]) -> String {
    match records {
        [] => "no filesystem side effects observed".to_owned(),
        [record] => format!(
            "observed filesystem side effect: {} `{}`",
            record.kind, record.path
        ),
        [first, ..] => format!(
            "observed {} filesystem side effects, including {} `{}`",
            records.len(),
            first.kind,
            first.path
        ),
    }
}

pub(super) fn issue_command_rank(command: &IssueCommand) -> (u8, usize) {
    let state_rank = match command.state.as_str() {
        "not_in_shape_catalog" if command.path.is_empty() => 0,
        "runtime_confirmed" => 1,
        "precondition_blocked" => 2,
        "unconfirmed" => 4,
        _ => 3,
    };
    (state_rank, command.path.len())
}

fn evidence_reference_rank(reference: &str, evidence: &EvidenceSummary) -> u8 {
    let event_id = reference.split_once(':').map_or(reference, |(id, _)| id);
    let suffix = reference
        .split_once(':')
        .map_or("", |(_, suffix)| suffix)
        .to_ascii_lowercase();
    if suffix.contains("auth_required")
        || suffix.contains("precondition")
        || suffix.contains("blocked")
        || suffix.contains("output mode probe")
    {
        return 0;
    }
    if evidence
        .side_effects
        .iter()
        .any(|record| record.evidence == event_id)
    {
        return 1;
    }
    if evidence
        .processes
        .get(event_id)
        .is_some_and(|process| !process.path.is_empty())
    {
        return 2;
    }
    if suffix.contains("usage") {
        return 3;
    }
    if suffix.contains("layout") {
        return 4;
    }
    5
}
