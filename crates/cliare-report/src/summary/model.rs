use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::artifacts::{
    IssueArtifact, IssueCommandArtifact, OutputContractArtifact, SummaryArtifacts, artifact_paths,
};
use crate::report_evidence::ProcessEvidence;
use crate::report_format::{command_path_label, output_mode_label, shell_arg};

const FINDING_EVIDENCE_EXCERPT_LIMIT: usize = 2;

#[derive(Debug, Clone, Serialize)]
pub struct MeasurementSummaryPacket {
    pub schema_version: &'static str,
    pub artifact_dir: PathBuf,
    pub target: TargetBrief,
    pub score: ScoreBrief,
    pub subscores: BTreeMap<String, MetricScore>,
    pub traversal: TraversalBrief,
    pub command_surface: CommandSurfaceBrief,
    pub agent_navigation_status: String,
    pub agent_navigation: Vec<AgentNavigationBrief>,
    pub top_findings: Vec<FindingBrief>,
    pub interpretation: Vec<String>,
    pub caveats: Vec<String>,
    pub next_actions: Vec<String>,
    pub source_artifacts: SourceArtifactBrief,
}

impl MeasurementSummaryPacket {
    pub(super) fn build(
        artifact_dir: &Path,
        artifacts: SummaryArtifacts,
        max_findings: usize,
        max_examples: usize,
    ) -> Self {
        let source_artifacts = SourceArtifactBrief::from_dir(artifact_dir);
        let target = TargetBrief {
            requested: artifacts.scorecard.target.requested.display().to_string(),
            resolved: artifacts.scorecard.target.resolved.display().to_string(),
            binary_sha256: artifacts.scorecard.target.binary_sha256.clone(),
        };
        let score = ScoreBrief::from(&artifacts.scorecard.score);
        let subscores = artifacts
            .scorecard
            .subscores
            .iter()
            .map(|(name, subscore)| {
                (
                    name.clone(),
                    MetricScore {
                        score: subscore.score,
                        status: subscore.status.clone(),
                    },
                )
            })
            .collect();
        let traversal = TraversalBrief::from_coverage(&artifacts.scorecard.coverage);
        let command_surface = CommandSurfaceBrief::from_artifacts(&artifacts);
        let agent_navigation_status = artifacts.scorecard.agent_navigation.status.clone();
        let agent_navigation =
            agent_navigation_briefs(&artifacts.scorecard.agent_navigation.dimensions);
        let top_findings = finding_briefs(
            &artifacts.issues,
            max_findings,
            max_examples,
            &artifacts.evidence.processes,
        );
        let caveats = caveats(&artifacts, &agent_navigation);
        let interpretation = interpretation(&artifacts, &agent_navigation, &top_findings);
        let next_actions = next_actions(&top_findings, &artifacts);

        Self {
            schema_version: "cliare.measurement-summary.v1",
            artifact_dir: artifact_dir.to_path_buf(),
            target,
            score,
            subscores,
            traversal,
            command_surface,
            agent_navigation_status,
            agent_navigation,
            top_findings,
            interpretation,
            caveats,
            next_actions,
            source_artifacts,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetBrief {
    pub requested: String,
    pub resolved: String,
    pub binary_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreBrief {
    pub total: f64,
    pub maintainer_readiness: f64,
    pub shape_confidence: f64,
    pub measured_weight: f64,
    pub max_weight: f64,
    pub model: String,
    pub status: String,
}

impl From<&super::artifacts::ScoreSummaryArtifact> for ScoreBrief {
    fn from(score: &super::artifacts::ScoreSummaryArtifact) -> Self {
        Self {
            total: score.total,
            maintainer_readiness: score.maintainer_readiness,
            shape_confidence: score.shape_confidence,
            measured_weight: score.measured_weight,
            max_weight: score.max_weight,
            model: score.model.clone(),
            status: score.status.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricScore {
    pub score: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraversalBrief {
    pub profile_complete: bool,
    pub stop_reason: String,
    pub observed_depth: usize,
    pub max_depth: usize,
    pub probes_completed: usize,
    pub max_probes: usize,
    pub frontier_remaining: usize,
    pub budget_exhausted: bool,
    pub sandbox_profile: String,
    pub env_policy: String,
}

impl TraversalBrief {
    fn from_coverage(coverage: &super::artifacts::CoverageArtifact) -> Self {
        Self {
            profile_complete: coverage.traversal_complete,
            stop_reason: coverage.traversal_stop_reason.clone(),
            observed_depth: coverage.observed_max_depth,
            max_depth: coverage.max_depth,
            probes_completed: coverage.probes_completed,
            max_probes: coverage.max_probes,
            frontier_remaining: coverage.frontier_remaining,
            budget_exhausted: coverage.budget_exhausted,
            sandbox_profile: coverage.sandbox_profile.clone(),
            env_policy: coverage.sandbox_env_policy.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandSurfaceBrief {
    pub discovered: usize,
    pub total: usize,
    pub ready: usize,
    pub conditional: usize,
    pub needs_fixture: usize,
    pub blocked: usize,
    pub candidate: usize,
    pub runtime_confirmed: usize,
    pub precondition_blocked: usize,
    pub parser_extraction_rate: f64,
    pub help_text_probes: usize,
    pub help_text_probes_with_shape: usize,
    pub help_text_probes_without_shape: usize,
    pub help_text_probes_not_recognized: usize,
    pub flags_discovered: usize,
    pub output_contracts_discovered: usize,
    pub machine_readable_output_contracts: usize,
    pub output_mode_probes_completed: usize,
    pub output_mode_parse_successes: usize,
    pub output_mode_precondition_blocked: usize,
    pub precondition_blocked_probes: usize,
    pub auth_required_probes: usize,
    pub local_context_required_probes: usize,
    pub fixture_required_probes: usize,
    pub actionable_precondition_probes: usize,
    pub precondition_recovery_rate: f64,
    pub side_effect_files_total: usize,
    pub side_effect_probe_count: usize,
    pub credential_like_side_effects: usize,
}

impl CommandSurfaceBrief {
    fn from_artifacts(artifacts: &SummaryArtifacts) -> Self {
        let summary = &artifacts.command_index.summary;
        let coverage = &artifacts.scorecard.coverage;
        Self {
            discovered: coverage.commands_discovered,
            total: artifacts.command_index.commands.len(),
            ready: summary.ready,
            conditional: summary.conditional,
            needs_fixture: summary.needs_fixture,
            blocked: summary.blocked,
            candidate: summary.candidate,
            runtime_confirmed: coverage.commands_runtime_confirmed,
            precondition_blocked: coverage.commands_precondition_blocked,
            parser_extraction_rate: coverage.parser_extraction_rate,
            help_text_probes: coverage.help_text_probes,
            help_text_probes_with_shape: coverage.help_text_probes_with_shape,
            help_text_probes_without_shape: coverage.help_text_probes_without_shape,
            help_text_probes_not_recognized: coverage.help_text_probes_not_recognized,
            flags_discovered: coverage.flags_discovered,
            output_contracts_discovered: coverage.output_contracts_discovered,
            machine_readable_output_contracts: coverage.machine_readable_output_contracts,
            output_mode_probes_completed: coverage.output_mode_probes_completed,
            output_mode_parse_successes: coverage.output_mode_parse_successes,
            output_mode_precondition_blocked: coverage.output_mode_precondition_blocked,
            precondition_blocked_probes: coverage.precondition_blocked_probes,
            auth_required_probes: coverage.auth_required_probes,
            local_context_required_probes: coverage.local_context_required_probes,
            fixture_required_probes: coverage.fixture_required_probes,
            actionable_precondition_probes: coverage.actionable_precondition_probes,
            precondition_recovery_rate: coverage.precondition_recovery_rate,
            side_effect_files_total: coverage.side_effect_files_total,
            side_effect_probe_count: coverage.side_effect_probe_count,
            credential_like_side_effects: coverage.credential_like_side_effects,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentNavigationBrief {
    pub capability: String,
    pub score: Option<f64>,
    pub numerator: usize,
    pub denominator: usize,
    pub status: String,
    pub rationale: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindingBrief {
    pub id: String,
    pub status: String,
    pub severity: String,
    pub category: String,
    pub assessment: String,
    pub meaning: String,
    pub suggested_remedy: String,
    pub affected_count: usize,
    pub associated_commands: Vec<FindingExample>,
    pub evidence_excerpts: Vec<EvidenceExcerptBrief>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceExcerptBrief {
    pub reference: String,
    pub command: String,
    pub status: String,
    pub stream: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindingExample {
    pub command: String,
    pub state: String,
    pub reason: String,
    pub required_positionals: Vec<String>,
    pub preconditions: Vec<String>,
    pub output_contracts: Vec<OutputContractBrief>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputContractBrief {
    pub mode: String,
    pub flag_name: Option<String>,
    pub status: String,
    pub diagnostic: Option<String>,
    pub skip_reason: Option<String>,
    pub suggested_validation: Option<String>,
}

impl From<&OutputContractArtifact> for OutputContractBrief {
    fn from(contract: &OutputContractArtifact) -> Self {
        Self {
            mode: contract.mode.clone(),
            flag_name: contract.flag_name.clone(),
            status: contract.status.clone(),
            diagnostic: contract.diagnostic.clone(),
            skip_reason: contract.skip_reason.clone(),
            suggested_validation: contract.suggested_validation.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceArtifactBrief {
    pub artifact_dir: PathBuf,
    pub scorecard: PathBuf,
    pub command_index: PathBuf,
    pub evidence: PathBuf,
    pub issues: PathBuf,
}

impl SourceArtifactBrief {
    fn from_dir(artifact_dir: &Path) -> Self {
        let paths = artifact_paths(artifact_dir);
        Self {
            artifact_dir: paths.artifact_dir,
            scorecard: paths.scorecard,
            command_index: paths.command_index,
            evidence: paths.evidence,
            issues: paths.issues,
        }
    }
}

fn agent_navigation_briefs(
    dimensions: &BTreeMap<String, super::artifacts::AgentNavigationMetricArtifact>,
) -> Vec<AgentNavigationBrief> {
    dimensions
        .iter()
        .map(|(capability, metric)| AgentNavigationBrief {
            capability: capability.clone(),
            score: metric.score,
            numerator: metric.numerator,
            denominator: metric.denominator,
            status: metric.status.clone(),
            rationale: metric.rationale.clone(),
            limitations: metric.limitations.clone(),
        })
        .collect()
}

fn finding_briefs(
    issues: &[IssueArtifact],
    max_findings: usize,
    max_examples: usize,
    processes: &BTreeMap<String, ProcessEvidence>,
) -> Vec<FindingBrief> {
    issues
        .iter()
        .filter(|issue| issue.status == "open")
        .take(max_findings)
        .map(|issue| FindingBrief {
            id: issue.id.clone(),
            status: issue.status.clone(),
            severity: issue.severity.clone(),
            category: issue.category.clone(),
            assessment: issue.title.clone(),
            meaning: issue.impact.clone(),
            suggested_remedy: issue.recommendation.clone(),
            affected_count: issue.affected_commands.len(),
            associated_commands: issue_examples(&issue.affected_commands, max_examples),
            evidence_excerpts: issue_evidence_excerpts(issue, processes),
        })
        .collect()
}

fn issue_evidence_excerpts(
    issue: &IssueArtifact,
    processes: &BTreeMap<String, ProcessEvidence>,
) -> Vec<EvidenceExcerptBrief> {
    let mut seen = BTreeSet::new();
    let mut excerpts = Vec::new();

    for evidence in &issue.evidence {
        if evidence.kind != "process" {
            continue;
        }
        let Some(event_id) = evidence_event_id(&evidence.reference) else {
            continue;
        };
        if !seen.insert(event_id.to_owned()) {
            continue;
        }
        let Some(process) = processes.get(event_id) else {
            continue;
        };
        let Some((stream, text)) = process_output_excerpt(process) else {
            continue;
        };

        excerpts.push(EvidenceExcerptBrief {
            reference: evidence.reference.clone(),
            command: command_invocation_label(&process.argv),
            status: process.status.clone(),
            stream: stream.to_owned(),
            text: text.to_owned(),
        });
        if excerpts.len() >= FINDING_EVIDENCE_EXCERPT_LIMIT {
            break;
        }
    }

    excerpts
}

fn evidence_event_id(reference: &str) -> Option<&str> {
    let event_id = reference
        .split_once(':')
        .map_or(reference, |(event_id, _)| event_id);
    (!event_id.is_empty()).then_some(event_id)
}

fn process_output_excerpt(process: &ProcessEvidence) -> Option<(&'static str, &str)> {
    process
        .stderr_excerpt
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .map(|text| ("stderr", text))
        .or_else(|| {
            process
                .stdout_excerpt
                .as_deref()
                .filter(|text| !text.trim().is_empty())
                .map(|text| ("stdout", text))
        })
}

fn command_invocation_label(argv: &[String]) -> String {
    let Some((binary, args)) = argv.split_first() else {
        return "<none>".to_owned();
    };
    let binary = Path::new(binary)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(binary);
    std::iter::once(binary)
        .chain(args.iter().map(String::as_str))
        .map(shell_arg)
        .collect::<Vec<_>>()
        .join(" ")
}

fn issue_examples(commands: &[IssueCommandArtifact], max_examples: usize) -> Vec<FindingExample> {
    commands
        .iter()
        .take(max_examples)
        .map(|command| FindingExample {
            command: command_path_label(&command.path),
            state: command.state.clone(),
            reason: command.reason.clone(),
            required_positionals: command.required_positionals.clone(),
            preconditions: command.preconditions.clone(),
            output_contracts: command
                .output_contracts
                .iter()
                .map(OutputContractBrief::from)
                .collect(),
        })
        .collect()
}

fn interpretation(
    artifacts: &SummaryArtifacts,
    agent_navigation: &[AgentNavigationBrief],
    top_findings: &[FindingBrief],
) -> Vec<String> {
    let mut lines = Vec::new();
    let score = &artifacts.scorecard.score;
    let coverage = &artifacts.scorecard.coverage;
    let surface = &artifacts.command_index.summary;

    lines.push(format!(
        "Score is {:.0}/100, while harness shape confidence is {:.0}/100.",
        score.total, score.shape_confidence
    ));
    if score.total - score.shape_confidence >= 15.0 {
        lines.push(
            "The lower harness confidence means the CLI is usable in many places, but agents still lack enough reliable shape evidence for broad automatic routing.".to_owned(),
        );
    }

    if coverage.traversal_complete {
        lines.push(format!(
            "Traversal converged after {} probes with no remaining frontier.",
            coverage.probes_completed
        ));
    } else {
        lines.push(format!(
            "Traversal is incomplete: stop reason `{}` with {} frontier item(s) remaining.",
            coverage.traversal_stop_reason, coverage.frontier_remaining
        ));
    }

    lines.push(format!(
        "Command surface: {} ready, {} conditional, {} fixture-needed, {} blocked, and {} candidate-only commands.",
        surface.ready, surface.conditional, surface.needs_fixture, surface.blocked, surface.candidate
    ));

    if let Some(metric) = weakest_measured_metric(agent_navigation) {
        lines.push(format!(
            "Weakest measured navigation capability is `{}` at {:.0}/100.",
            metric.capability,
            metric.score.unwrap_or(0.0)
        ));
    }

    if coverage.machine_readable_output_contracts > 0 {
        lines.push(format!(
            "Machine-readable output validation is {}/{} parse successes.",
            coverage.output_mode_parse_successes, coverage.machine_readable_output_contracts
        ));
    }

    if let Some(issue) = top_findings.first() {
        lines.push(format!(
            "Top finding is `{}`: {}.",
            issue.id, issue.assessment
        ));
    }

    lines
}

fn weakest_measured_metric(metrics: &[AgentNavigationBrief]) -> Option<&AgentNavigationBrief> {
    metrics
        .iter()
        .filter(|metric| metric.status == "measured")
        .filter(|metric| metric.score.is_some())
        .min_by(|left, right| {
            left.score
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.score.unwrap_or(f64::INFINITY))
        })
}

fn caveats(artifacts: &SummaryArtifacts, metrics: &[AgentNavigationBrief]) -> Vec<String> {
    let mut caveats = artifacts.scorecard.agent_navigation.limitations.to_vec();
    let coverage = &artifacts.scorecard.coverage;

    if coverage.sandbox_profile == "host" {
        caveats.push(
            "Host execution mode does not prove clean filesystem behavior; side-effect safety is a manual review input.".to_owned(),
        );
    }
    if metrics
        .iter()
        .any(|metric| metric.capability == "example_validity" && metric.status == "not_measured")
    {
        caveats.push(
            "Example syntax is not validated yet; examples remain hints, not scored contracts."
                .to_owned(),
        );
    }
    if coverage.auth_required_probes > 0 {
        caveats.push(
            "Review auth-required classifications against raw evidence when diagnostics mention auth-related flags; option names can look like preconditions.".to_owned(),
        );
    }

    dedupe(caveats)
}

fn next_actions(top_findings: &[FindingBrief], artifacts: &SummaryArtifacts) -> Vec<String> {
    let mut actions = Vec::new();
    for finding in top_findings.iter().take(3) {
        actions.push(format!("{}: {}", finding.id, finding.suggested_remedy));
    }
    if artifacts.scorecard.coverage.sandbox_profile == "host" {
        actions.push(
            "Rerun in isolated execution mode when filesystem side-effect confidence matters."
                .to_owned(),
        );
    }
    if artifacts.command_index.summary.candidate > 0 {
        actions.push(
            "Inspect candidate-only commands in command-index.json before exposing them to automatic routing."
                .to_owned(),
        );
    }
    dedupe(actions)
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }
    deduped
}

pub(super) fn contract_label(contract: &OutputContractBrief) -> String {
    let flag = contract
        .flag_name
        .as_deref()
        .map_or_else(|| "<no flag>".to_owned(), str::to_owned);
    format!(
        "{} via `{}` ({})",
        output_mode_label(&contract.mode),
        flag,
        contract.status
    )
}
