use std::collections::{BTreeMap, BTreeSet};

use super::recommendations::persona_priority;
use super::util::{unique_command_paths, unique_strings};
use super::*;
use cliare_policy::path_classification;

pub(super) fn action_items(persona: Persona, artifacts: &MeasuredArtifacts) -> Vec<ActionItem> {
    let mut items = grouped_gap_action_items(persona, artifacts);
    let gap_kinds = artifacts
        .shape
        .gaps
        .iter()
        .map(|gap| gap.kind.as_str())
        .collect::<BTreeSet<_>>();

    for finding in &artifacts.scorecard.findings {
        if finding_is_subsumed_by_gap(finding, &gap_kinds) {
            continue;
        }
        let category = ActionCategory::from_dimension(&finding.dimension);
        let command_paths = command_paths_for_finding(finding);
        let evidence = evidence_for_finding(finding, artifacts);
        let affected_count = affected_count_for_finding(finding, artifacts);
        items.push(action_item(ActionItemInput {
            persona,
            id: finding.id.clone(),
            severity: ActionSeverity::from_scorecard(&finding.severity),
            category,
            title: finding.title.clone(),
            detail: finding.detail.clone(),
            recommendation: finding.recommendation.clone(),
            command_paths,
            evidence,
            dimension: Some(finding.dimension.clone()),
            affected_count,
        }));
    }

    if !artifacts.scorecard.coverage.traversal_complete {
        items.push(action_item(ActionItemInput {
            persona,
            id: "coverage.traversal_incomplete".to_owned(),
            severity: ActionSeverity::Medium,
            category: ActionCategory::Coverage,
            title: "Traversal did not cover the full observed frontier".to_owned(),
            detail: format!(
                "Traversal stopped with reason `{}`; frontier remaining is {}; skipped by depth {}; skipped by probe budget {}.",
                artifacts.scorecard.coverage.traversal_stop_reason,
                artifacts.scorecard.coverage.frontier_remaining,
                artifacts.scorecard.coverage.candidates_skipped_by_depth,
                artifacts.scorecard.coverage.probes_skipped_by_budget
            ),
            recommendation: "Rerun with a deeper profile or larger probe budget before treating the surface as fully characterized."
                .to_owned(),
            command_paths: Vec::new(),
            evidence: Vec::new(),
            dimension: Some("coverage".to_owned()),
            affected_count: Some(artifacts.scorecard.coverage.frontier_remaining),
        }));
    }

    match persona {
        Persona::Platform => items.push(action_item(ActionItemInput {
            persona,
            id: "platform.policy_gate.configure".to_owned(),
            severity: ActionSeverity::Low,
            category: ActionCategory::Policy,
            title: "Configure an explicit guard policy for platform CI".to_owned(),
            detail: "The packet was generated from measurement artifacts; platform enforcement requires `cliare guard` with a baseline and policy file.".to_owned(),
            recommendation: "Add a `cliare.policy.json` file with score thresholds and side-effect rules, then run `cliare guard` in CI.".to_owned(),
            command_paths: Vec::new(),
            evidence: Vec::new(),
            dimension: Some("policy".to_owned()),
            affected_count: None,
        })),
        Persona::Oss | Persona::Devrel => items.push(action_item(ActionItemInput {
            persona,
            id: "publishing.claims.keep_provisional".to_owned(),
            severity: ActionSeverity::Low,
            category: ActionCategory::Publishing,
            title: "Keep public score claims bounded to local evidence".to_owned(),
            detail: "The current scorecard is useful for CI feedback and public transparency, but it is not a certified leaderboard entry.".to_owned(),
            recommendation: "Publish the scorecard with its profile, binary fingerprint, score model, and traversal status; avoid certified-ranking language until calibration profiles are finalized.".to_owned(),
            command_paths: Vec::new(),
            evidence: Vec::new(),
            dimension: Some("publishing".to_owned()),
            affected_count: None,
        })),
        Persona::Research => items.push(action_item(ActionItemInput {
            persona,
            id: "research.calibration.label_evidence".to_owned(),
            severity: ActionSeverity::Low,
            category: ActionCategory::Calibration,
            title: "Label evidence before treating the run as calibration data".to_owned(),
            detail: "The packet preserves model versions, budgets, evidence references, and command health, but calibration requires truth labels and independent quality checks.".to_owned(),
            recommendation: "Attach human-verified truth labels for command existence, output contracts, preconditions, and side effects before using this run to tune score weights.".to_owned(),
            command_paths: Vec::new(),
            evidence: Vec::new(),
            dimension: Some("calibration".to_owned()),
            affected_count: None,
        })),
        Persona::Maintainer | Persona::Harness | Persona::Security => {}
    }

    items.sort_by(|left, right| {
        left.persona_priority
            .cmp(&right.persona_priority)
            .then(left.severity.cmp(&right.severity))
            .then(left.id.cmp(&right.id))
    });
    items
}

fn grouped_gap_action_items(persona: Persona, artifacts: &MeasuredArtifacts) -> Vec<ActionItem> {
    let mut groups: BTreeMap<String, Vec<&ShapeGap>> = BTreeMap::new();
    for gap in &artifacts.shape.gaps {
        groups.entry(gap.kind.clone()).or_default().push(gap);
    }

    let finding_ids = artifacts
        .scorecard
        .findings
        .iter()
        .map(|finding| finding.id.as_str())
        .collect::<BTreeSet<_>>();

    groups
        .into_iter()
        .map(|(kind, gaps)| {
            let category = category_for_gap(&kind);
            let command_paths = gaps
                .iter()
                .map(|gap| gap.command_path.clone())
                .collect::<Vec<_>>();
            let evidence = gaps
                .iter()
                .flat_map(|gap| gap.evidence.iter().cloned())
                .collect::<Vec<_>>();
            let reason = gaps
                .first()
                .map_or("observed shape gap".to_owned(), |gap| gap.reason.clone());
            let count = unique_command_paths(command_paths.clone()).len();
            action_item(ActionItemInput {
                persona,
                id: format!("shape.gap.{kind}"),
                severity: severity_for_gap(&kind, &finding_ids),
                category,
                title: grouped_title_for_gap(&kind, count),
                detail: format!(
                    "{} command path{} affected. Reason pattern: {}.",
                    count,
                    plural_suffix(count),
                    reason
                ),
                recommendation: recommendation_for_gap(&kind).to_owned(),
                command_paths,
                evidence,
                dimension: Some(category.label().to_owned()),
                affected_count: Some(count),
            })
        })
        .collect()
}

struct ActionItemInput {
    persona: Persona,
    id: String,
    severity: ActionSeverity,
    category: ActionCategory,
    title: String,
    detail: String,
    recommendation: String,
    command_paths: Vec<Vec<String>>,
    evidence: Vec<String>,
    dimension: Option<String>,
    affected_count: Option<usize>,
}

fn action_item(input: ActionItemInput) -> ActionItem {
    let command_paths = unique_command_paths(input.command_paths);
    let evidence = unique_strings(input.evidence);
    let sample_command_paths = command_paths
        .iter()
        .take(COMMAND_SAMPLE_LIMIT)
        .cloned()
        .collect::<Vec<_>>();
    let evidence_count = evidence.len();
    let evidence = evidence
        .into_iter()
        .take(ACTION_EVIDENCE_LIMIT)
        .collect::<Vec<_>>();

    ActionItem {
        id: input.id,
        severity: input.severity,
        category: input.category,
        title: input.title,
        detail: input.detail,
        recommendation: input.recommendation,
        affected_count: input.affected_count.unwrap_or(command_paths.len()),
        sample_command_paths,
        command_paths,
        evidence_count,
        evidence,
        dimension: input.dimension,
        persona_priority: persona_priority(input.persona, input.category),
    }
}

fn finding_is_subsumed_by_gap(finding: &FindingArtifact, gap_kinds: &BTreeSet<&str>) -> bool {
    match finding.id.as_str() {
        "finding.precondition.auth_required" | "finding.precondition.runtime_blocked" => {
            gap_kinds.contains("precondition_blocked")
        }
        "finding.discovery.low_runtime_confirmation" => gap_kinds.contains("existence_unconfirmed"),
        "finding.output.unparseable_mode" => gap_kinds.contains("output_mode_parse_failed"),
        "finding.grammar.unconfirmed_arity" => {
            gap_kinds.contains("flags_unknown") || gap_kinds.contains("argument_arity_unknown")
        }
        "finding.recovery.invalid_probe_acceptance" => {
            gap_kinds.contains("invalid_child_diagnostics_unknown")
                || gap_kinds.contains("invalid_flag_diagnostics_unknown")
        }
        _ => false,
    }
}

fn command_paths_for_finding(finding: &FindingArtifact) -> Vec<Vec<String>> {
    match finding.id.as_str() {
        "finding.safety.safe_probe_side_effects"
        | "finding.safety.credential_like_side_effects" => Vec::new(),
        _ => Vec::new(),
    }
}

fn affected_count_for_finding(
    finding: &FindingArtifact,
    artifacts: &MeasuredArtifacts,
) -> Option<usize> {
    match finding.id.as_str() {
        "finding.safety.safe_probe_side_effects" => Some(artifacts.evidence.side_effects.len()),
        "finding.safety.credential_like_side_effects" => Some(
            artifacts
                .evidence
                .side_effects
                .iter()
                .filter(|record| path_classification::credential_like_path_text(&record.path))
                .count(),
        ),
        _ => None,
    }
}

pub(super) fn evidence_for_finding(
    finding: &FindingArtifact,
    artifacts: &MeasuredArtifacts,
) -> Vec<String> {
    match finding.id.as_str() {
        "finding.safety.safe_probe_side_effects" => artifacts
            .evidence
            .side_effects
            .iter()
            .map(|record| record.evidence.clone())
            .collect(),
        "finding.safety.credential_like_side_effects" => artifacts
            .evidence
            .side_effects
            .iter()
            .filter(|record| path_classification::credential_like_path_text(&record.path))
            .map(|record| record.evidence.clone())
            .collect(),
        _ => Vec::new(),
    }
}

fn category_for_gap(kind: &str) -> ActionCategory {
    match kind {
        "existence_unconfirmed"
        | "help_unavailable"
        | "alternate_help_form_unavailable"
        | "precondition_blocked" => ActionCategory::Discovery,
        "flags_unknown" | "argument_arity_unknown" => ActionCategory::Grammar,
        "invalid_child_diagnostics_unknown" | "invalid_flag_diagnostics_unknown" => {
            ActionCategory::Recovery
        }
        "output_mode_unprobed" | "output_mode_unvalidated" | "output_mode_parse_failed" => {
            ActionCategory::Output
        }
        _ => ActionCategory::Coverage,
    }
}

fn severity_for_gap(kind: &str, finding_ids: &BTreeSet<&str>) -> ActionSeverity {
    match kind {
        "precondition_blocked" | "output_mode_parse_failed" => ActionSeverity::High,
        "existence_unconfirmed"
            if finding_ids.contains("finding.discovery.low_runtime_confirmation") =>
        {
            ActionSeverity::High
        }
        "existence_unconfirmed"
        | "help_unavailable"
        | "output_mode_unprobed"
        | "output_mode_unvalidated" => ActionSeverity::Medium,
        _ => ActionSeverity::Low,
    }
}

fn grouped_title_for_gap(kind: &str, count: usize) -> String {
    match kind {
        "existence_unconfirmed" => format!(
            "{count} command candidate{} need runtime confirmation",
            plural_suffix(count)
        ),
        "help_unavailable" => format!(
            "{count} command{} did not expose usable help",
            plural_suffix(count)
        ),
        "alternate_help_form_unavailable" => format!(
            "{count} command{} lack optional `help <path>` compatibility",
            plural_suffix(count)
        ),
        "precondition_blocked" => format!(
            "{count} command{} were blocked by runtime preconditions",
            plural_suffix(count)
        ),
        "flags_unknown" => format!(
            "{count} command{} have incomplete flag grammar",
            plural_suffix(count)
        ),
        "argument_arity_unknown" => format!(
            "{count} command{} have incomplete argument grammar",
            plural_suffix(count)
        ),
        "invalid_child_diagnostics_unknown" => format!(
            "{count} command{} {} clearer invalid-subcommand diagnostics",
            plural_suffix(count),
            need_verb(count)
        ),
        "invalid_flag_diagnostics_unknown" => format!(
            "{count} command{} {} clearer invalid-flag diagnostics",
            plural_suffix(count),
            need_verb(count)
        ),
        "output_mode_unprobed" => format!(
            "{count} advertised output mode{} still {} validation",
            plural_suffix(count),
            need_verb(count)
        ),
        "output_mode_unvalidated" => format!(
            "{count} advertised output mode{} need command-local validation",
            plural_suffix(count)
        ),
        "output_mode_parse_failed" => format!(
            "{count} advertised output mode{} did not parse",
            plural_suffix(count)
        ),
        _ => format!(
            "{count} observed shape gap{} need review",
            plural_suffix(count)
        ),
    }
}

fn plural_suffix(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn need_verb(count: usize) -> &'static str {
    if count == 1 { "needs" } else { "need" }
}

fn recommendation_for_gap(kind: &str) -> &'static str {
    match kind {
        "existence_unconfirmed" => {
            "Expose consistent help for this command or increase runtime probe budget."
        }
        "help_unavailable" => {
            "Make command-specific help available without side effects and with CI-safe output."
        }
        "alternate_help_form_unavailable" => {
            "Treat direct `<command> --help` as canonical; add `help <command path>` compatibility only if it is cheap and useful for agent navigation."
        }
        "precondition_blocked" => {
            "Document required runtime preconditions separately from command existence, and keep help paths available where practical."
        }
        "flags_unknown" => "Document flag value requirements directly in command help.",
        "argument_arity_unknown" => {
            "Add explicit usage syntax that identifies required, optional, and variadic arguments."
        }
        "invalid_child_diagnostics_unknown" => {
            "Reject unknown subcommands with clear nonzero diagnostics."
        }
        "invalid_flag_diagnostics_unknown" => {
            "Reject unknown flags with clear nonzero diagnostics and suggestions where possible."
        }
        "output_mode_unprobed" => {
            "Provide safe fixture operands, a documented dry-run/sample invocation, or machine-readable metadata so the advertised output contract can be validated without guessing."
        }
        "output_mode_unvalidated" => {
            "Provide safe data-producing fixtures, command-local examples, or explicit metadata that scopes inherited output flags to commands that actually emit machine data."
        }
        "output_mode_parse_failed" => {
            "Ensure advertised JSON or YAML modes produce parseable machine output under safe probes."
        }
        _ => {
            "Review the evidence references and improve the command surface where the gap is confirmed."
        }
    }
}
