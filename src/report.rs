use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;

use crate::artifact_guide::{self, ArtifactGuideSummary};
use crate::artifacts::{
    EVIDENCE_JSONL, ISSUES_JSON, ISSUES_MD, MeasurementArtifactPaths, SCORECARD_JSON, SHAPE_JSON,
};
use crate::cli::{ReportArgs, ReportFormat, ReportPersona};
use crate::context;
use crate::error::{CliareError, Result};
use crate::fingerprint::TargetFingerprint;
use crate::path_classification;

const PACKET_SCHEMA_VERSION: &str = "cliare.persona-outcome.v1";
const ISSUE_LEDGER_SCHEMA_VERSION: &str = "cliare.issue-ledger.v1";
const ACTION_EVIDENCE_LIMIT: usize = 32;
const COMMAND_SAMPLE_LIMIT: usize = 5;
const PERSONA_FINDING_LIMIT: usize = 5;
const PERSONA_COMMAND_SAMPLE_LIMIT: usize = 3;
const PERSONA_EVIDENCE_SAMPLE_LIMIT: usize = 3;
const COMMAND_GUIDANCE_SAMPLE_LIMIT: usize = 10;
const EVIDENCE_SAMPLE_LIMIT: usize = 50;
const TOP_ISSUE_LIMIT: usize = 12;

#[derive(Debug, Clone)]
pub struct ReportSummary {
    pub persona: Persona,
    pub format: ReportFormat,
    pub artifact_dir: PathBuf,
    pub markdown_path: Option<PathBuf>,
    pub json_path: Option<PathBuf>,
    pub readme_path: Option<PathBuf>,
    pub agent_skill_path: Option<PathBuf>,
    pub score_total: f64,
    pub action_items: usize,
    pub command_health: usize,
    stdout: String,
}

#[derive(Debug, Clone)]
pub struct PersonaArtifactSummary {
    pub issues_markdown_path: PathBuf,
    pub issues_json_path: PathBuf,
    pub persona_markdown_paths: Vec<PathBuf>,
    pub persona_json_paths: Vec<PathBuf>,
}

impl PersonaArtifactSummary {
    pub fn persona_count(&self) -> usize {
        self.persona_markdown_paths
            .len()
            .max(self.persona_json_paths.len())
    }
}

impl ReportSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.stdout
    }
}

pub async fn report(args: ReportArgs) -> Result<ReportSummary> {
    let persona = Persona::from(args.persona);
    let artifact_dir =
        context::resolve_measurement_dir(&args.out, args.context.as_deref(), "cliare report")
            .await?;
    let artifacts = MeasuredArtifacts::read(&artifact_dir).await?;
    let issue_ledger = IssueLedger::build(&artifact_dir, &artifacts);
    let packet = PersonaOutcomePacket::build(persona, &artifact_dir, &artifacts, &issue_ledger);
    let markdown = render_markdown(&packet);
    let json =
        serde_json::to_string_pretty(&packet).map_err(CliareError::SerializePersonaOutcome)?;
    let issues_markdown = render_issue_ledger_markdown(&issue_ledger);
    let issues_json = serde_json::to_string_pretty(&issue_ledger)
        .map_err(CliareError::SerializePersonaOutcome)?;

    let (markdown_path, json_path, guide_artifacts) = if args.write {
        let markdown_path = artifact_dir.join(format!("persona-{}.md", persona.label()));
        let json_path = artifact_dir.join(format!("persona-{}.json", persona.label()));
        let issues_markdown_path = artifact_dir.join("issues.md");
        let issues_json_path = artifact_dir.join("issues.json");
        write_persona_artifact(&markdown_path, markdown.as_bytes()).await?;
        write_persona_artifact(&json_path, json.as_bytes()).await?;
        write_persona_artifact(&issues_markdown_path, issues_markdown.as_bytes()).await?;
        write_persona_artifact(&issues_json_path, issues_json.as_bytes()).await?;
        let guide_artifacts = artifact_guide::write_measurement_guides(&artifact_dir).await?;
        (Some(markdown_path), Some(json_path), Some(guide_artifacts))
    } else {
        (None, None, None)
    };

    let stdout = if args.write {
        render_written_summary(
            &packet,
            markdown_path.as_ref(),
            json_path.as_ref(),
            guide_artifacts.as_ref(),
        )
    } else {
        match args.format {
            ReportFormat::Markdown => markdown,
            ReportFormat::Json => format!("{json}\n"),
        }
    };

    Ok(ReportSummary {
        persona,
        format: args.format,
        artifact_dir,
        markdown_path,
        json_path,
        readme_path: guide_artifacts
            .as_ref()
            .map(|artifacts| artifacts.readme_path.clone()),
        agent_skill_path: guide_artifacts
            .as_ref()
            .map(|artifacts| artifacts.agent_skill_path.clone()),
        score_total: packet.summary.score,
        action_items: packet.action_items.len(),
        command_health: packet.command_health.len(),
        stdout,
    })
}

pub async fn write_all_persona_reports(out_dir: &Path) -> Result<PersonaArtifactSummary> {
    let artifacts = MeasuredArtifacts::read(out_dir).await?;
    let issue_ledger = IssueLedger::build(out_dir, &artifacts);
    let issues_markdown = render_issue_ledger_markdown(&issue_ledger);
    let issues_json = serde_json::to_string_pretty(&issue_ledger)
        .map_err(CliareError::SerializePersonaOutcome)?;
    let issues_markdown_path = out_dir.join(ISSUES_MD);
    let issues_json_path = out_dir.join(ISSUES_JSON);
    write_persona_artifact(&issues_markdown_path, issues_markdown.as_bytes()).await?;
    write_persona_artifact(&issues_json_path, issues_json.as_bytes()).await?;

    let mut persona_markdown_paths = Vec::with_capacity(Persona::all().len());
    let mut persona_json_paths = Vec::with_capacity(Persona::all().len());
    for persona in Persona::all() {
        let packet = PersonaOutcomePacket::build(*persona, out_dir, &artifacts, &issue_ledger);
        let markdown = render_markdown(&packet);
        let json =
            serde_json::to_string_pretty(&packet).map_err(CliareError::SerializePersonaOutcome)?;
        let markdown_path = out_dir.join(format!("persona-{}.md", persona.label()));
        let json_path = out_dir.join(format!("persona-{}.json", persona.label()));
        write_persona_artifact(&markdown_path, markdown.as_bytes()).await?;
        write_persona_artifact(&json_path, json.as_bytes()).await?;
        persona_markdown_paths.push(markdown_path);
        persona_json_paths.push(json_path);
    }

    Ok(PersonaArtifactSummary {
        issues_markdown_path,
        issues_json_path,
        persona_markdown_paths,
        persona_json_paths,
    })
}

async fn write_persona_artifact(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::write(path, bytes)
        .await
        .map_err(|source| CliareError::WritePersonaOutcome {
            path: path.to_path_buf(),
            source,
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Persona {
    Maintainer,
    Harness,
    Platform,
    Security,
    Oss,
    Devrel,
    Research,
}

const ALL_PERSONAS: [Persona; 7] = [
    Persona::Maintainer,
    Persona::Harness,
    Persona::Platform,
    Persona::Security,
    Persona::Oss,
    Persona::Devrel,
    Persona::Research,
];

impl Persona {
    pub fn all() -> &'static [Self] {
        &ALL_PERSONAS
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Maintainer => "maintainer",
            Self::Harness => "harness",
            Self::Platform => "platform",
            Self::Security => "security",
            Self::Oss => "oss",
            Self::Devrel => "devrel",
            Self::Research => "research",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Maintainer => "CLI Maintainer",
            Self::Harness => "Agent Harness Builder",
            Self::Platform => "Platform Engineering",
            Self::Security => "Security and Governance",
            Self::Oss => "Open-Source Maintainer",
            Self::Devrel => "Developer Relations",
            Self::Research => "Benchmark and Research",
        }
    }

    fn primary_question(self) -> &'static str {
        match self {
            Self::Maintainer => "What should change in the CLI to improve agent readiness?",
            Self::Harness => "Which command subset is ready to expose to agents?",
            Self::Platform => "Can this CLI pass an internal automation quality gate?",
            Self::Security => "What runtime evidence matters for approval or restriction?",
            Self::Oss => "Is this scorecard ready to publish credibly?",
            Self::Devrel => "Which public readiness claims are supported by evidence?",
            Self::Research => "Can this run support replay, labeling, and calibration?",
        }
    }
}

impl From<ReportPersona> for Persona {
    fn from(value: ReportPersona) -> Self {
        match value {
            ReportPersona::Maintainer => Self::Maintainer,
            ReportPersona::Harness => Self::Harness,
            ReportPersona::Platform => Self::Platform,
            ReportPersona::Security => Self::Security,
            ReportPersona::Oss => Self::Oss,
            ReportPersona::Devrel => Self::Devrel,
            ReportPersona::Research => Self::Research,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PersonaOutcomePacket {
    schema_version: &'static str,
    persona: Persona,
    persona_title: &'static str,
    primary_question: &'static str,
    target: TargetFingerprint,
    source_artifacts: SourceArtifacts,
    summary: OutcomeSummary,
    run_recommendations: Vec<RunRecommendation>,
    top_issues: Vec<Issue>,
    action_items: Vec<ActionItem>,
    command_health: Vec<CommandHealth>,
    score: ScoreSection,
    coverage: CoverageSection,
    evidence_summary: EvidenceSummaryPacket,
    notes: Vec<OutcomeNote>,
}

impl PersonaOutcomePacket {
    fn build(
        persona: Persona,
        artifact_dir: &Path,
        artifacts: &MeasuredArtifacts,
        issue_ledger: &IssueLedger,
    ) -> Self {
        let command_health = command_health(&artifacts.shape);
        let summary = OutcomeSummary::from_artifacts(artifacts, command_health.len());
        let action_items = action_items(persona, artifacts);
        let run_recommendations = run_recommendations(persona, &artifacts.scorecard, artifact_dir);
        let notes = notes(persona, &artifacts.scorecard);
        let top_issues = top_issues_for_persona(persona, issue_ledger);

        Self {
            schema_version: PACKET_SCHEMA_VERSION,
            persona,
            persona_title: persona.title(),
            primary_question: persona.primary_question(),
            target: artifacts.scorecard.target.clone(),
            source_artifacts: SourceArtifacts::new(artifact_dir),
            summary,
            run_recommendations,
            top_issues,
            action_items,
            command_health,
            score: ScoreSection::from(&artifacts.scorecard),
            coverage: CoverageSection::from(&artifacts.scorecard.coverage),
            evidence_summary: EvidenceSummaryPacket::from(&artifacts.evidence),
            notes,
        }
    }
}

#[derive(Debug, Serialize)]
struct SourceArtifacts {
    artifact_dir: PathBuf,
    evidence: PathBuf,
    shape: PathBuf,
    command_index: PathBuf,
    command_index_markdown: PathBuf,
    scorecard: PathBuf,
}

impl SourceArtifacts {
    fn new(artifact_dir: &Path) -> Self {
        let paths = MeasurementArtifactPaths::from_dir(artifact_dir);
        Self {
            artifact_dir: artifact_dir.to_path_buf(),
            evidence: paths.evidence,
            shape: paths.shape,
            command_index: paths.command_index_json,
            command_index_markdown: paths.command_index_markdown,
            scorecard: paths.scorecard,
        }
    }
}

#[derive(Debug, Serialize)]
struct OutcomeSummary {
    score: f64,
    score_model: String,
    score_status: String,
    measured_weight: f64,
    max_weight: f64,
    findings: usize,
    shape_gaps: usize,
    commands_discovered: usize,
    commands_runtime_confirmed: usize,
    commands_precondition_blocked: usize,
    command_health_entries: usize,
    output_contracts_discovered: usize,
    machine_readable_output_contracts: usize,
    output_mode_parse_successes: usize,
    side_effect_files_total: usize,
    credential_like_side_effects: usize,
    precondition_blocked_probes: usize,
    auth_required_probes: usize,
    local_context_required_probes: usize,
    fixture_required_probes: usize,
    observed_max_depth: usize,
    max_depth: usize,
    max_probes: usize,
    probes_completed: usize,
    frontier_remaining: usize,
    budget_exhausted: bool,
    traversal_stop_reason: String,
    traversal_complete: bool,
}

impl OutcomeSummary {
    fn from_artifacts(artifacts: &MeasuredArtifacts, command_health_entries: usize) -> Self {
        let score = &artifacts.scorecard.score;
        let coverage = &artifacts.scorecard.coverage;
        Self {
            score: score.total,
            score_model: score.model.clone(),
            score_status: score.status.clone(),
            measured_weight: score.measured_weight,
            max_weight: score.max_weight,
            findings: artifacts.scorecard.findings.len(),
            shape_gaps: artifacts.shape.gaps.len(),
            commands_discovered: coverage.commands_discovered,
            commands_runtime_confirmed: coverage.commands_runtime_confirmed,
            commands_precondition_blocked: coverage.commands_precondition_blocked,
            command_health_entries,
            output_contracts_discovered: coverage.output_contracts_discovered,
            machine_readable_output_contracts: coverage.machine_readable_output_contracts,
            output_mode_parse_successes: coverage.output_mode_parse_successes,
            side_effect_files_total: coverage.side_effect_files_total,
            credential_like_side_effects: coverage.credential_like_side_effects,
            precondition_blocked_probes: coverage.precondition_blocked_probes,
            auth_required_probes: coverage.auth_required_probes,
            local_context_required_probes: coverage.local_context_required_probes,
            fixture_required_probes: coverage.fixture_required_probes,
            observed_max_depth: coverage.observed_max_depth,
            max_depth: coverage.max_depth,
            max_probes: coverage.max_probes,
            probes_completed: coverage.probes_completed,
            frontier_remaining: coverage.frontier_remaining,
            budget_exhausted: coverage.budget_exhausted,
            traversal_stop_reason: coverage.traversal_stop_reason.clone(),
            traversal_complete: coverage.traversal_complete,
        }
    }
}

#[derive(Debug, Serialize)]
struct ScoreSection {
    total: f64,
    measured_weight: f64,
    max_weight: f64,
    model: String,
    status: String,
    subscores: BTreeMap<String, ScoreSubscorePacket>,
}

impl From<&ScorecardArtifact> for ScoreSection {
    fn from(scorecard: &ScorecardArtifact) -> Self {
        Self {
            total: scorecard.score.total,
            measured_weight: scorecard.score.measured_weight,
            max_weight: scorecard.score.max_weight,
            model: scorecard.score.model.clone(),
            status: scorecard.score.status.clone(),
            subscores: scorecard
                .subscores
                .iter()
                .map(|(dimension, subscore)| {
                    (
                        dimension.clone(),
                        ScoreSubscorePacket {
                            score: subscore.score,
                            weight: subscore.weight,
                            status: subscore.status.clone(),
                            rationale: subscore.rationale.clone(),
                        },
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ScoreSubscorePacket {
    score: Option<f64>,
    weight: f64,
    status: String,
    rationale: String,
}

#[derive(Debug, Serialize)]
struct CoverageSection {
    commands_discovered: usize,
    commands_runtime_confirmed: usize,
    commands_precondition_blocked: usize,
    command_confirmation_rate: f64,
    flags_discovered: usize,
    output_contracts_discovered: usize,
    machine_readable_output_contracts: usize,
    output_mode_probes_completed: usize,
    output_mode_parse_successes: usize,
    output_mode_precondition_blocked: usize,
    side_effect_files_created: usize,
    side_effect_files_modified: usize,
    side_effect_files_deleted: usize,
    side_effect_files_total: usize,
    side_effect_probe_count: usize,
    credential_like_side_effects: usize,
    avg_command_confidence: f64,
    avg_flag_confidence: f64,
    observed_max_depth: usize,
    traversal_profile: String,
    max_depth: usize,
    max_probes: usize,
    min_expected_value: u16,
    concurrency_limit: usize,
    traversal_rounds: usize,
    probes_scheduled: usize,
    probes_completed: usize,
    probes_cancelled: usize,
    probes_timed_out: usize,
    probes_failed_to_spawn: usize,
    precondition_blocked_probes: usize,
    auth_required_probes: usize,
    local_context_required_probes: usize,
    fixture_required_probes: usize,
    frontier_remaining: usize,
    highest_pending_expected_value: Option<u16>,
    candidates_skipped_by_depth: usize,
    candidates_skipped_by_convergence: usize,
    probes_skipped_by_budget: usize,
    budget_exhausted: bool,
    traversal_stop_reason: String,
    traversal_complete: bool,
}

impl From<&CoverageArtifact> for CoverageSection {
    fn from(coverage: &CoverageArtifact) -> Self {
        Self {
            commands_discovered: coverage.commands_discovered,
            commands_runtime_confirmed: coverage.commands_runtime_confirmed,
            commands_precondition_blocked: coverage.commands_precondition_blocked,
            command_confirmation_rate: coverage.command_confirmation_rate,
            flags_discovered: coverage.flags_discovered,
            output_contracts_discovered: coverage.output_contracts_discovered,
            machine_readable_output_contracts: coverage.machine_readable_output_contracts,
            output_mode_probes_completed: coverage.output_mode_probes_completed,
            output_mode_parse_successes: coverage.output_mode_parse_successes,
            output_mode_precondition_blocked: coverage.output_mode_precondition_blocked,
            side_effect_files_created: coverage.side_effect_files_created,
            side_effect_files_modified: coverage.side_effect_files_modified,
            side_effect_files_deleted: coverage.side_effect_files_deleted,
            side_effect_files_total: coverage.side_effect_files_total,
            side_effect_probe_count: coverage.side_effect_probe_count,
            credential_like_side_effects: coverage.credential_like_side_effects,
            avg_command_confidence: coverage.avg_command_confidence,
            avg_flag_confidence: coverage.avg_flag_confidence,
            observed_max_depth: coverage.observed_max_depth,
            traversal_profile: coverage.traversal_profile.clone(),
            max_depth: coverage.max_depth,
            max_probes: coverage.max_probes,
            min_expected_value: coverage.min_expected_value,
            concurrency_limit: coverage.concurrency_limit,
            traversal_rounds: coverage.traversal_rounds,
            probes_scheduled: coverage.probes_scheduled,
            probes_completed: coverage.probes_completed,
            probes_cancelled: coverage.probes_cancelled,
            probes_timed_out: coverage.probes_timed_out,
            probes_failed_to_spawn: coverage.probes_failed_to_spawn,
            precondition_blocked_probes: coverage.precondition_blocked_probes,
            auth_required_probes: coverage.auth_required_probes,
            local_context_required_probes: coverage.local_context_required_probes,
            fixture_required_probes: coverage.fixture_required_probes,
            frontier_remaining: coverage.frontier_remaining,
            highest_pending_expected_value: coverage.highest_pending_expected_value,
            candidates_skipped_by_depth: coverage.candidates_skipped_by_depth,
            candidates_skipped_by_convergence: coverage.candidates_skipped_by_convergence,
            probes_skipped_by_budget: coverage.probes_skipped_by_budget,
            budget_exhausted: coverage.budget_exhausted,
            traversal_stop_reason: coverage.traversal_stop_reason.clone(),
            traversal_complete: coverage.traversal_complete,
        }
    }
}

#[derive(Debug, Serialize)]
struct RunRecommendation {
    id: String,
    priority: u16,
    command: String,
    purpose: String,
    when_to_use: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ActionSeverity {
    High,
    Medium,
    Low,
}

impl ActionSeverity {
    fn from_scorecard(value: &str) -> Self {
        match value {
            "high" => Self::High,
            "medium" => Self::Medium,
            _ => Self::Low,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ActionCategory {
    Discovery,
    Grammar,
    Execution,
    Output,
    Safety,
    Recovery,
    Coverage,
    Policy,
    Publishing,
    Calibration,
}

impl ActionCategory {
    fn from_dimension(value: &str) -> Self {
        match value {
            "discovery" => Self::Discovery,
            "grammar" => Self::Grammar,
            "execution" => Self::Execution,
            "output" => Self::Output,
            "safety" => Self::Safety,
            "recovery" => Self::Recovery,
            _ => Self::Coverage,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Grammar => "grammar",
            Self::Execution => "execution",
            Self::Output => "output",
            Self::Safety => "safety",
            Self::Recovery => "recovery",
            Self::Coverage => "coverage",
            Self::Policy => "policy",
            Self::Publishing => "publishing",
            Self::Calibration => "calibration",
        }
    }
}

#[derive(Debug, Serialize)]
struct ActionItem {
    id: String,
    severity: ActionSeverity,
    category: ActionCategory,
    title: String,
    detail: String,
    recommendation: String,
    affected_count: usize,
    sample_command_paths: Vec<Vec<String>>,
    command_paths: Vec<Vec<String>>,
    evidence_count: usize,
    evidence: Vec<String>,
    dimension: Option<String>,
    persona_priority: u16,
}

#[derive(Debug, Serialize)]
struct IssueLedger {
    schema_version: &'static str,
    target: TargetFingerprint,
    source_artifacts: SourceArtifacts,
    summary: IssueLedgerSummary,
    issues: Vec<Issue>,
}

impl IssueLedger {
    fn build(artifact_dir: &Path, artifacts: &MeasuredArtifacts) -> Self {
        let command_index = artifacts
            .shape
            .commands
            .iter()
            .map(|command| (command.path.clone(), command))
            .collect::<BTreeMap<_, _>>();
        let mut output_contracts_by_command =
            BTreeMap::<Vec<String>, Vec<&ShapeOutputContract>>::new();
        for contract in &artifacts.shape.output_contracts {
            output_contracts_by_command
                .entry(contract.command_path.clone())
                .or_default()
                .push(contract);
        }
        let mut gaps_by_command = BTreeMap::<Vec<String>, Vec<&ShapeGap>>::new();
        for gap in &artifacts.shape.gaps {
            gaps_by_command
                .entry(gap.command_path.clone())
                .or_default()
                .push(gap);
        }

        let mut issues = action_items(Persona::Maintainer, artifacts)
            .into_iter()
            .map(|item| {
                issue_from_action_item(
                    item,
                    artifacts,
                    &command_index,
                    &gaps_by_command,
                    &output_contracts_by_command,
                )
            })
            .collect::<Vec<_>>();
        issues.sort_by(|left, right| {
            left.severity
                .cmp(&right.severity)
                .then(left.category.cmp(&right.category))
                .then(left.id.cmp(&right.id))
        });

        let summary = IssueLedgerSummary::from_issues(&issues);
        Self {
            schema_version: ISSUE_LEDGER_SCHEMA_VERSION,
            target: artifacts.scorecard.target.clone(),
            source_artifacts: SourceArtifacts::new(artifact_dir),
            summary,
            issues,
        }
    }
}

#[derive(Debug, Serialize)]
struct IssueLedgerSummary {
    issues_total: usize,
    high: usize,
    medium: usize,
    low: usize,
    affected_commands: usize,
    requires_fixtures: usize,
    blocked_by_preconditions: usize,
}

impl IssueLedgerSummary {
    fn from_issues(issues: &[Issue]) -> Self {
        let mut affected_commands = BTreeSet::<Vec<String>>::new();
        let mut high = 0_usize;
        let mut medium = 0_usize;
        let mut low = 0_usize;
        let mut requires_fixtures = 0_usize;
        let mut blocked_by_preconditions = 0_usize;

        for issue in issues {
            match issue.severity {
                ActionSeverity::High => high += 1,
                ActionSeverity::Medium => medium += 1,
                ActionSeverity::Low => low += 1,
            }
            if issue.confidence == IssueConfidence::NeedsFixture {
                requires_fixtures += 1;
            }
            if issue.confidence == IssueConfidence::Blocked {
                blocked_by_preconditions += 1;
            }
            for command in &issue.affected_commands {
                affected_commands.insert(command.path.clone());
            }
        }

        Self {
            issues_total: issues.len(),
            high,
            medium,
            low,
            affected_commands: affected_commands.len(),
            requires_fixtures,
            blocked_by_preconditions,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct Issue {
    id: String,
    status: &'static str,
    severity: ActionSeverity,
    category: ActionCategory,
    confidence: IssueConfidence,
    title: String,
    impact: String,
    why_it_matters: String,
    recommendation: String,
    verification: IssueVerification,
    affected_commands: Vec<IssueCommand>,
    evidence: Vec<IssueEvidence>,
    personas: Vec<Persona>,
    score_dimensions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum IssueConfidence {
    Observed,
    Blocked,
    Inferred,
    NeedsFixture,
    Advisory,
}

impl IssueConfidence {
    fn label(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Blocked => "blocked",
            Self::Inferred => "inferred",
            Self::NeedsFixture => "needs_fixture",
            Self::Advisory => "advisory",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct IssueVerification {
    command: String,
    expected_change: String,
}

#[derive(Debug, Clone, Serialize)]
struct IssueCommand {
    path: Vec<String>,
    argv: Vec<String>,
    state: String,
    confidence: Option<f64>,
    summary: Option<String>,
    required_positionals: Vec<String>,
    output_contracts: Vec<IssueOutputContract>,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct IssueOutputContract {
    mode: String,
    flag_name: String,
    argv_fragment: Vec<String>,
    status: String,
    probed: bool,
    parse_success: bool,
    precondition_blocked: bool,
    diagnostic: Option<String>,
    help_behavior: Option<String>,
    skip_reason: Option<String>,
    suggested_validation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct IssueEvidence {
    kind: String,
    reference: String,
    detail: String,
    probe_id: Option<String>,
    intent: Option<String>,
    scope: String,
    argv: Vec<String>,
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interpretation: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    side_effects: Vec<IssueSideEffect>,
}

#[derive(Debug, Clone, Serialize)]
struct IssueSideEffect {
    operation: String,
    region: String,
    path: String,
    credential_like: bool,
    size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CommandReadinessState {
    Ready,
    Blocked,
    Incomplete,
    Unconfirmed,
}

#[derive(Debug, Serialize)]
struct CommandHealth {
    id: String,
    path: Vec<String>,
    argv: Vec<String>,
    summary: Option<String>,
    confidence: f64,
    runtime_state: String,
    readiness_state: CommandReadinessState,
    preconditions: Vec<String>,
    flags_discovered: usize,
    output_contracts: Vec<CommandOutputContract>,
    gaps: Vec<CommandGap>,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CommandOutputContract {
    mode: String,
    flag_name: String,
    argv_fragment: Vec<String>,
    advertised: bool,
    probed: bool,
    parse_success: bool,
    precondition_blocked: bool,
    observed_kind: Option<String>,
    diagnostic: Option<String>,
    help_probed: bool,
    help_behavior: Option<String>,
    help_parse_success: bool,
    help_diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CommandGap {
    kind: String,
    reason: String,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
struct EvidenceSummaryPacket {
    probes_scheduled: usize,
    processes_completed: usize,
    probe_failures_total: usize,
    probe_failure_samples: Vec<ProbeFailure>,
    side_effects_total: usize,
    side_effect_samples: Vec<SideEffectRecord>,
}

impl From<&EvidenceSummary> for EvidenceSummaryPacket {
    fn from(summary: &EvidenceSummary) -> Self {
        Self {
            probes_scheduled: summary.probes_scheduled,
            processes_completed: summary.processes_completed,
            probe_failures_total: summary.probe_failures.len(),
            probe_failure_samples: summary
                .probe_failures
                .iter()
                .take(EVIDENCE_SAMPLE_LIMIT)
                .cloned()
                .collect(),
            side_effects_total: summary.side_effects.len(),
            side_effect_samples: summary
                .side_effects
                .iter()
                .take(EVIDENCE_SAMPLE_LIMIT)
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ProbeFailure {
    probe_id: String,
    intent: Option<String>,
    path: Vec<String>,
    argv: Vec<String>,
    status: String,
    evidence: String,
}

#[derive(Debug, Clone, Serialize)]
struct SideEffectRecord {
    evidence: String,
    probe_id: String,
    intent: Option<String>,
    command_path: Vec<String>,
    argv: Vec<String>,
    kind: String,
    region: String,
    path: String,
    size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct OutcomeNote {
    level: &'static str,
    text: String,
}

struct MeasuredArtifacts {
    scorecard: ScorecardArtifact,
    shape: ShapeArtifact,
    evidence: EvidenceSummary,
}

impl MeasuredArtifacts {
    async fn read(out_dir: &Path) -> Result<Self> {
        let scorecard = read_json::<ScorecardArtifact>(&out_dir.join(SCORECARD_JSON)).await?;
        let shape = read_json::<ShapeArtifact>(&out_dir.join(SHAPE_JSON)).await?;
        let evidence = EvidenceSummary::read(&out_dir.join(EVIDENCE_JSONL)).await?;
        Ok(Self {
            scorecard,
            shape,
            evidence,
        })
    }
}

async fn read_json<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = fs::read(path)
        .await
        .map_err(|source| CliareError::ReadReportArtifact {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseReportArtifact {
        path: path.to_path_buf(),
        source,
    })
}

#[derive(Debug, Deserialize)]
struct ScorecardArtifact {
    target: TargetFingerprint,
    score: ScoreSummaryArtifact,
    subscores: BTreeMap<String, SubscoreArtifact>,
    coverage: CoverageArtifact,
    findings: Vec<FindingArtifact>,
}

#[derive(Debug, Deserialize)]
struct ScoreSummaryArtifact {
    total: f64,
    measured_weight: f64,
    max_weight: f64,
    model: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct SubscoreArtifact {
    score: Option<f64>,
    weight: f64,
    status: String,
    rationale: String,
}

#[derive(Debug, Deserialize)]
struct CoverageArtifact {
    commands_discovered: usize,
    commands_runtime_confirmed: usize,
    commands_precondition_blocked: usize,
    command_confirmation_rate: f64,
    flags_discovered: usize,
    output_contracts_discovered: usize,
    machine_readable_output_contracts: usize,
    output_mode_probes_completed: usize,
    output_mode_parse_successes: usize,
    output_mode_precondition_blocked: usize,
    side_effect_files_created: usize,
    side_effect_files_modified: usize,
    side_effect_files_deleted: usize,
    side_effect_files_total: usize,
    side_effect_probe_count: usize,
    credential_like_side_effects: usize,
    avg_command_confidence: f64,
    avg_flag_confidence: f64,
    observed_max_depth: usize,
    traversal_profile: String,
    max_depth: usize,
    max_probes: usize,
    min_expected_value: u16,
    concurrency_limit: usize,
    traversal_rounds: usize,
    probes_scheduled: usize,
    probes_completed: usize,
    probes_cancelled: usize,
    probes_timed_out: usize,
    probes_failed_to_spawn: usize,
    precondition_blocked_probes: usize,
    #[serde(default)]
    auth_required_probes: usize,
    #[serde(default)]
    local_context_required_probes: usize,
    #[serde(default)]
    fixture_required_probes: usize,
    frontier_remaining: usize,
    highest_pending_expected_value: Option<u16>,
    candidates_skipped_by_depth: usize,
    candidates_skipped_by_convergence: usize,
    probes_skipped_by_budget: usize,
    budget_exhausted: bool,
    traversal_stop_reason: String,
    traversal_complete: bool,
}

#[derive(Debug, Deserialize)]
struct FindingArtifact {
    id: String,
    dimension: String,
    severity: String,
    title: String,
    detail: String,
    recommendation: String,
}

#[derive(Debug, Deserialize)]
struct ShapeArtifact {
    commands: Vec<ShapeCommand>,
    flags: Vec<ShapeFlag>,
    output_contracts: Vec<ShapeOutputContract>,
    gaps: Vec<ShapeGap>,
}

#[derive(Debug, Deserialize)]
struct ShapeCommand {
    id: String,
    path: Vec<String>,
    argv: Vec<String>,
    summary: Option<String>,
    positionals: Vec<ShapePositionalArgument>,
    confidence: f64,
    runtime_state: String,
    preconditions: Vec<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ShapePositionalArgument {
    name: String,
    required: bool,
    #[allow(dead_code)]
    variadic: bool,
    #[allow(dead_code)]
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ShapeFlag {
    command_path: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ShapeOutputContract {
    command_path: Vec<String>,
    mode: String,
    flag_name: String,
    argv_fragment: Vec<String>,
    advertised: bool,
    probed: bool,
    parse_success: bool,
    precondition_blocked: bool,
    observed_kind: Option<String>,
    diagnostic: Option<String>,
    #[serde(default)]
    help_probed: bool,
    #[serde(default)]
    help_behavior: Option<String>,
    #[serde(default)]
    help_parse_success: bool,
    #[serde(default)]
    help_diagnostic: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ShapeGap {
    kind: String,
    command_path: Vec<String>,
    reason: String,
    evidence: Vec<String>,
}

#[derive(Debug, Default)]
struct EvidenceSummary {
    probes_scheduled: usize,
    processes_completed: usize,
    scheduled: BTreeMap<String, ScheduledProbe>,
    processes: BTreeMap<String, ProcessEvidence>,
    probe_failures: Vec<ProbeFailure>,
    side_effects: Vec<SideEffectRecord>,
}

impl EvidenceSummary {
    async fn read(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path)
                .await
                .map_err(|source| CliareError::ReadReportArtifact {
                    path: path.to_path_buf(),
                    source,
                })?;
        let mut summary = Self::default();

        for (line_index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let value: Value =
                serde_json::from_str(line).map_err(|source| CliareError::ParseReportEvidence {
                    path: path.to_path_buf(),
                    line: line_index + 1,
                    source,
                })?;
            match value["kind"].as_str() {
                Some("probe_scheduled") => summary.record_scheduled(&value),
                Some("process_completed") => summary.record_process(&value),
                _ => {}
            }
        }

        Ok(summary)
    }

    fn record_scheduled(&mut self, value: &Value) {
        let payload = &value["payload"];
        let Some(probe_id) = payload["probe_id"].as_str() else {
            return;
        };
        self.probes_scheduled += 1;
        self.scheduled.insert(
            probe_id.to_owned(),
            ScheduledProbe {
                intent: payload["intent"].as_str().map(str::to_owned),
                path: string_array(&payload["path"]),
                argv: string_array(&payload["argv"]),
            },
        );
    }

    fn record_process(&mut self, value: &Value) {
        let payload = &value["payload"];
        let Some(probe_id) = payload["probe_id"].as_str() else {
            return;
        };
        self.processes_completed += 1;

        let scheduled = self.scheduled.get(probe_id);
        let argv = string_array(&payload["argv"]);
        let command_path = scheduled.map_or_else(Vec::new, |probe| probe.path.clone());
        let intent = scheduled.and_then(|probe| probe.intent.clone());
        let status = status_label(&payload["status"]);
        let status_state = payload["status"]["state"].as_str();
        let evidence_id = value["event_id"].as_str().unwrap_or_default().to_owned();
        self.processes.insert(
            evidence_id.clone(),
            ProcessEvidence {
                probe_id: probe_id.to_owned(),
                intent: intent.clone(),
                path: command_path.clone(),
                argv: if argv.is_empty() {
                    scheduled.map_or_else(Vec::new, |probe| probe.argv.clone())
                } else {
                    argv.clone()
                },
                status: status.clone(),
            },
        );
        if matches!(status_state, Some("timed_out" | "spawn_failed")) {
            self.probe_failures.push(ProbeFailure {
                probe_id: probe_id.to_owned(),
                intent: intent.clone(),
                path: command_path.clone(),
                argv: if argv.is_empty() {
                    scheduled.map_or_else(Vec::new, |probe| probe.argv.clone())
                } else {
                    argv.clone()
                },
                status: status.clone(),
                evidence: evidence_id.clone(),
            });
        }

        let Some(changes) = payload["side_effects"]["changes"].as_array() else {
            return;
        };
        for change in changes {
            let Some(change_path) = change["path"].as_str() else {
                continue;
            };
            self.side_effects.push(SideEffectRecord {
                evidence: evidence_id.clone(),
                probe_id: probe_id.to_owned(),
                intent: intent.clone(),
                command_path: command_path.clone(),
                argv: if argv.is_empty() {
                    scheduled.map_or_else(Vec::new, |probe| probe.argv.clone())
                } else {
                    argv.clone()
                },
                kind: change["kind"].as_str().unwrap_or("unknown").to_owned(),
                region: change["region"].as_str().unwrap_or("unknown").to_owned(),
                path: change_path.to_owned(),
                size_bytes: change["size_bytes"].as_u64(),
            });
        }
    }
}

#[derive(Debug)]
struct ScheduledProbe {
    intent: Option<String>,
    path: Vec<String>,
    argv: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProcessEvidence {
    probe_id: String,
    intent: Option<String>,
    path: Vec<String>,
    argv: Vec<String>,
    status: String,
}

impl ProcessEvidence {
    fn summary(&self) -> String {
        format!(
            "probe `{}` for {} completed with `{}`",
            self.probe_id,
            self.scope_label(),
            self.status
        )
    }

    fn scope_label(&self) -> String {
        let path = if self.path.is_empty() {
            return "root command".to_owned();
        } else {
            self.path.join(" ")
        };
        format!("command `{path}`")
    }
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_owned))
        .collect()
}

fn status_label(status: &Value) -> String {
    match status["state"].as_str() {
        Some("exited") => match status["code"].as_i64() {
            Some(code) => format!("exited:{code}"),
            None => "exited:none".to_owned(),
        },
        Some("timed_out") => "timed_out".to_owned(),
        Some("spawn_failed") => status["error"].as_str().map_or_else(
            || "spawn_failed".to_owned(),
            |error| format!("spawn_failed:{error}"),
        ),
        Some(other) => other.to_owned(),
        None => "unknown".to_owned(),
    }
}

fn command_health(shape: &ShapeArtifact) -> Vec<CommandHealth> {
    let mut flags_by_command: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for flag in &shape.flags {
        *flags_by_command
            .entry(flag.command_path.clone())
            .or_default() += 1;
    }

    let mut contracts_by_command: BTreeMap<Vec<String>, Vec<CommandOutputContract>> =
        BTreeMap::new();
    for contract in &shape.output_contracts {
        contracts_by_command
            .entry(contract.command_path.clone())
            .or_default()
            .push(CommandOutputContract {
                mode: contract.mode.clone(),
                flag_name: contract.flag_name.clone(),
                argv_fragment: contract.argv_fragment.clone(),
                advertised: contract.advertised,
                probed: contract.probed,
                parse_success: contract.parse_success,
                precondition_blocked: contract.precondition_blocked,
                observed_kind: contract.observed_kind.clone(),
                diagnostic: contract.diagnostic.clone(),
                help_probed: contract.help_probed,
                help_behavior: contract.help_behavior.clone(),
                help_parse_success: contract.help_parse_success,
                help_diagnostic: contract.help_diagnostic.clone(),
            });
    }

    let mut gaps_by_command: BTreeMap<Vec<String>, Vec<CommandGap>> = BTreeMap::new();
    for gap in &shape.gaps {
        gaps_by_command
            .entry(gap.command_path.clone())
            .or_default()
            .push(CommandGap {
                kind: gap.kind.clone(),
                reason: gap.reason.clone(),
                evidence: gap.evidence.clone(),
            });
    }

    shape
        .commands
        .iter()
        .map(|command| {
            let gaps = gaps_by_command.remove(&command.path).unwrap_or_default();
            let readiness_state = readiness_state(command, &gaps);
            CommandHealth {
                id: command.id.clone(),
                path: command.path.clone(),
                argv: command.argv.clone(),
                summary: command.summary.clone(),
                confidence: command.confidence,
                runtime_state: command.runtime_state.clone(),
                readiness_state,
                preconditions: command.preconditions.clone(),
                flags_discovered: flags_by_command
                    .get(&command.path)
                    .copied()
                    .unwrap_or_default(),
                output_contracts: contracts_by_command
                    .remove(&command.path)
                    .unwrap_or_default(),
                gaps,
                evidence: command.evidence.clone(),
            }
        })
        .collect()
}

fn readiness_state(command: &ShapeCommand, gaps: &[CommandGap]) -> CommandReadinessState {
    match command.runtime_state.as_str() {
        "runtime_confirmed" if gaps.is_empty() => CommandReadinessState::Ready,
        "runtime_confirmed" => CommandReadinessState::Incomplete,
        "precondition_blocked" => CommandReadinessState::Blocked,
        _ => CommandReadinessState::Unconfirmed,
    }
}

fn action_items(persona: Persona, artifacts: &MeasuredArtifacts) -> Vec<ActionItem> {
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

fn issue_from_action_item(
    item: ActionItem,
    artifacts: &MeasuredArtifacts,
    command_index: &BTreeMap<Vec<String>, &ShapeCommand>,
    gaps_by_command: &BTreeMap<Vec<String>, Vec<&ShapeGap>>,
    output_contracts_by_command: &BTreeMap<Vec<String>, Vec<&ShapeOutputContract>>,
) -> Issue {
    let confidence = issue_confidence(&item);
    let affected_commands = issue_commands(
        &item,
        command_index,
        gaps_by_command,
        output_contracts_by_command,
    );
    let evidence_references = issue_evidence_references(&item, artifacts);
    let evidence = issue_evidence(&evidence_references, &artifacts.evidence, item.category);
    let score_dimensions = item.dimension.clone().into_iter().collect::<Vec<_>>();
    let verification = issue_verification(&item, confidence, artifacts);

    Issue {
        id: item.id.clone().replace("shape.gap.", "issue."),
        status: "open",
        severity: item.severity,
        category: item.category,
        confidence,
        title: item.title,
        impact: issue_impact(item.category, confidence).to_owned(),
        why_it_matters: issue_why_it_matters(item.category).to_owned(),
        recommendation: item.recommendation,
        verification,
        affected_commands,
        evidence,
        personas: personas_for_issue(item.category, confidence),
        score_dimensions,
    }
}

fn issue_confidence(item: &ActionItem) -> IssueConfidence {
    if item.id.contains("output_mode_unprobed") || item.id.contains("output_mode_unvalidated") {
        return IssueConfidence::NeedsFixture;
    }
    if item.id.contains("precondition") || item.id.contains("auth_required") {
        return IssueConfidence::Blocked;
    }
    if item.id.contains("unavailable")
        || item.id.contains("unconfirmed")
        || item.id.contains("unknown")
    {
        return IssueConfidence::Inferred;
    }
    if matches!(
        item.category,
        ActionCategory::Publishing | ActionCategory::Calibration
    ) {
        return IssueConfidence::Advisory;
    }
    IssueConfidence::Observed
}

fn issue_commands(
    item: &ActionItem,
    command_index: &BTreeMap<Vec<String>, &ShapeCommand>,
    gaps_by_command: &BTreeMap<Vec<String>, Vec<&ShapeGap>>,
    output_contracts_by_command: &BTreeMap<Vec<String>, Vec<&ShapeOutputContract>>,
) -> Vec<IssueCommand> {
    let mut commands = item
        .command_paths
        .iter()
        .map(|path| {
            let command = command_index.get(path);
            let required_positionals = command_required_positionals(command.copied());
            let output_contracts = if item.category == ActionCategory::Output {
                output_contracts_by_command
                    .get(path)
                    .into_iter()
                    .flatten()
                    .map(|contract| issue_output_contract(contract, command.copied()))
                    .collect()
            } else {
                Vec::new()
            };
            IssueCommand {
                path: path.clone(),
                argv: command.map_or_else(Vec::new, |command| command.argv.clone()),
                state: command.map_or_else(
                    || "not_in_shape_catalog".to_owned(),
                    |command| command.runtime_state.clone(),
                ),
                confidence: command.map(|command| command.confidence),
                summary: command.and_then(|command| command.summary.clone()),
                required_positionals,
                reason: issue_command_reason(
                    path,
                    item,
                    gaps_by_command,
                    command.copied(),
                    &output_contracts,
                ),
                output_contracts,
            }
        })
        .collect::<Vec<_>>();
    commands.sort_by(|left, right| {
        issue_command_rank(left)
            .cmp(&issue_command_rank(right))
            .then(left.path.cmp(&right.path))
    });
    commands
}

fn issue_command_reason(
    path: &[String],
    item: &ActionItem,
    gaps_by_command: &BTreeMap<Vec<String>, Vec<&ShapeGap>>,
    command: Option<&ShapeCommand>,
    output_contracts: &[IssueOutputContract],
) -> String {
    if (item.id.contains("output_mode_unprobed") || item.id.contains("output_mode_unvalidated"))
        && !output_contracts.is_empty()
    {
        let contracts = output_contracts
            .iter()
            .map(|contract| {
                format!(
                    "{} via `{}`",
                    output_mode_label(&contract.mode),
                    shell_words(&contract.argv_fragment)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let required = command_required_positionals(command);
        if required.is_empty() {
            return format!(
                "Advertises {contracts}, but CLIARE did not runtime-probe this contract in the current run."
            );
        }
        return format!(
            "Advertises {contracts}, but CLIARE did not execute it because the command requires safe operand values for {}.",
            required
                .iter()
                .map(|name| format!("<{name}>"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    gaps_by_command
        .get(path)
        .and_then(|gaps| {
            gaps.iter()
                .copied()
                .find(|gap| item.id.ends_with(&gap.kind))
                .or_else(|| gaps.first().copied())
        })
        .map_or_else(|| item.detail.clone(), |gap| gap.reason.clone())
}

fn command_required_positionals(command: Option<&ShapeCommand>) -> Vec<String> {
    command
        .into_iter()
        .flat_map(|command| command.positionals.iter())
        .filter(|argument| argument.required)
        .map(|argument| argument.name.clone())
        .collect()
}

fn issue_output_contract(
    contract: &ShapeOutputContract,
    command: Option<&ShapeCommand>,
) -> IssueOutputContract {
    let required_positionals = command_required_positionals(command);
    let status = if contract.parse_success {
        "validated"
    } else if contract.precondition_blocked {
        "blocked"
    } else if contract.observed_kind.as_deref() == Some("help_text") {
        "help_text"
    } else if contract.probed {
        "probe_failed"
    } else if required_positionals.is_empty() {
        "unprobed"
    } else {
        "needs_fixture"
    };
    let skip_reason = if contract.observed_kind.as_deref() == Some("help_text") {
        Some(
            "The safe output-mode probe reached help text rather than a data-producing command path."
                .to_owned(),
        )
    } else if contract.probed {
        None
    } else if required_positionals.is_empty() {
        Some("CLIARE did not schedule this output probe in the current run.".to_owned())
    } else {
        Some(format!(
            "CLIARE avoided running `{}` without values for required operands {}.",
            shell_words(
                &command
                    .map_or_else(Vec::new, |command| command.argv.clone())
                    .into_iter()
                    .chain(contract.argv_fragment.clone())
                    .collect::<Vec<_>>()
            ),
            required_positionals
                .iter()
                .map(|name| format!("<{name}>"))
                .collect::<Vec<_>>()
                .join(" ")
        ))
    };
    let suggested_validation = if contract.observed_kind.as_deref() == Some("help_text") {
        Some(format!(
            "Validate `{}` on a safe invocation that produces data instead of command help.",
            shell_words(
                &command
                    .map_or_else(Vec::new, |command| command.argv.clone())
                    .into_iter()
                    .chain(contract.argv_fragment.clone())
                    .collect::<Vec<_>>()
            )
        ))
    } else if !contract.probed && !required_positionals.is_empty() {
        Some(format!(
            "Provide a safe fixture invocation for `{}` with {} plus `{}`.",
            shell_words(&command.map_or_else(Vec::new, |command| command.argv.clone())),
            required_positionals
                .iter()
                .map(|name| format!("<{name}>"))
                .collect::<Vec<_>>()
                .join(" "),
            shell_words(&contract.argv_fragment)
        ))
    } else {
        None
    };

    IssueOutputContract {
        mode: contract.mode.clone(),
        flag_name: contract.flag_name.clone(),
        argv_fragment: contract.argv_fragment.clone(),
        status: status.to_owned(),
        probed: contract.probed,
        parse_success: contract.parse_success,
        precondition_blocked: contract.precondition_blocked,
        diagnostic: contract.diagnostic.clone(),
        help_behavior: contract.help_behavior.clone(),
        skip_reason,
        suggested_validation,
    }
}

fn shell_words(words: &[String]) -> String {
    if words.is_empty() {
        "<none>".to_owned()
    } else {
        words.join(" ")
    }
}

fn output_mode_label(mode: &str) -> String {
    match mode {
        "json" => "JSON".to_owned(),
        "yaml" => "YAML".to_owned(),
        "table" => "table".to_owned(),
        "plain" => "plain text".to_owned(),
        other => other.to_owned(),
    }
}

fn issue_evidence_references(item: &ActionItem, artifacts: &MeasuredArtifacts) -> Vec<String> {
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

fn issue_evidence(
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

fn issue_command_rank(command: &IssueCommand) -> (u8, usize) {
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

fn issue_verification(
    item: &ActionItem,
    confidence: IssueConfidence,
    artifacts: &MeasuredArtifacts,
) -> IssueVerification {
    let target = shell_arg(&artifacts.scorecard.target.requested.display().to_string());
    let command = format!("cliare measure {target} --out .cliare --profile deep --refresh");
    let expected_change = match confidence {
        IssueConfidence::Observed if item.category == ActionCategory::Safety => {
            "The side-effect finding no longer appears in `issues.json` and the related score dimension improves."
        }
        IssueConfidence::Observed => {
            "The observed runtime finding no longer appears in `issues.json` and the related score dimension improves."
        }
        IssueConfidence::Blocked => {
            "The affected commands either become safely measurable or remain explicitly classified with documented runtime preconditions."
        }
        IssueConfidence::Inferred => {
            "The affected command candidates become runtime-confirmed, intentionally rejected, or disappear from the inferred shape."
        }
        IssueConfidence::NeedsFixture => {
            "The contract moves from unprobed to parse_success=true, blocked with a documented precondition, or explicitly fixture-required."
        }
        IssueConfidence::Advisory => {
            "The issue remains documented as a deliberate policy or publishing choice."
        }
    };

    IssueVerification {
        command,
        expected_change: format!("{} Source action: {}.", expected_change, item.id),
    }
}

fn issue_impact(category: ActionCategory, confidence: IssueConfidence) -> &'static str {
    match (category, confidence) {
        (ActionCategory::Output, IssueConfidence::NeedsFixture) => {
            "Agents and harnesses cannot rely on the advertised output contract until safe operands or fixtures validate it."
        }
        (ActionCategory::Output, _) => {
            "Agents need stable machine-readable output for routing, state inspection, and recovery."
        }
        (ActionCategory::Discovery, IssueConfidence::Blocked) => {
            "Clean CI and agent harnesses may be unable to distinguish command existence from configured account state."
        }
        (ActionCategory::Discovery, _) => {
            "Agents may miss commands or route to commands that are not actually available at runtime."
        }
        (ActionCategory::Grammar, _) => {
            "Agents cannot construct reliable invocations without clear operands, flag arity, and value expectations."
        }
        (ActionCategory::Execution, _) => {
            "Agents need execution behavior that is consistent across safe probes and real task invocations."
        }
        (ActionCategory::Recovery, _) => {
            "Agents depend on precise nonzero diagnostics to repair bad command attempts."
        }
        (ActionCategory::Safety, _) => {
            "Safe discovery paths should not write durable state unless the behavior is intentional and documented."
        }
        (ActionCategory::Coverage, _) => {
            "The current measurement does not fully characterize the observed surface."
        }
        (ActionCategory::Policy, _) => {
            "The organization needs explicit CI policy before enforcing readiness gates."
        }
        (ActionCategory::Publishing, _) => {
            "Public readiness claims should stay within what the measured evidence supports."
        }
        (ActionCategory::Calibration, _) => {
            "Calibration data requires labels and reproducible metadata before it can tune score authority."
        }
    }
}

fn issue_why_it_matters(category: ActionCategory) -> &'static str {
    match category {
        ActionCategory::Discovery => {
            "Discovery is the first contract an agent sees; ambiguity here propagates into every downstream plan."
        }
        ActionCategory::Grammar => {
            "Grammar quality determines whether an agent can build a command without trial-and-error."
        }
        ActionCategory::Execution => {
            "Execution behavior determines whether safe probes and real tasks behave consistently."
        }
        ActionCategory::Output => {
            "Machine-readable output is the main bridge from CLI behavior to agent state."
        }
        ActionCategory::Safety => {
            "Agent harnesses need to know what safe discovery does to the filesystem and environment."
        }
        ActionCategory::Recovery => {
            "Good diagnostics reduce retries, wrong repairs, and irreversible follow-up actions."
        }
        ActionCategory::Coverage => {
            "Coverage determines how much confidence the scorecard can honestly claim."
        }
        ActionCategory::Policy => "Policy turns measurement into a repeatable release gate.",
        ActionCategory::Publishing => {
            "Credible public reporting requires bounded claims and reproducible artifacts."
        }
        ActionCategory::Calibration => {
            "Research reuse depends on traceable, labeled, and versioned evidence."
        }
    }
}

fn personas_for_issue(category: ActionCategory, confidence: IssueConfidence) -> Vec<Persona> {
    let mut personas = match category {
        ActionCategory::Discovery | ActionCategory::Grammar | ActionCategory::Recovery => {
            vec![Persona::Maintainer, Persona::Harness, Persona::Platform]
        }
        ActionCategory::Output => vec![
            Persona::Maintainer,
            Persona::Harness,
            Persona::Platform,
            Persona::Oss,
            Persona::Devrel,
        ],
        ActionCategory::Safety => vec![Persona::Security, Persona::Harness, Persona::Platform],
        ActionCategory::Coverage => vec![Persona::Platform, Persona::Oss, Persona::Research],
        ActionCategory::Policy => vec![Persona::Platform],
        ActionCategory::Publishing => vec![Persona::Oss, Persona::Devrel],
        ActionCategory::Calibration => vec![Persona::Research],
        ActionCategory::Execution => vec![Persona::Maintainer, Persona::Harness],
    };
    if confidence == IssueConfidence::Blocked && !personas.contains(&Persona::Security) {
        personas.push(Persona::Security);
    }
    personas
}

fn top_issues_for_persona(persona: Persona, issue_ledger: &IssueLedger) -> Vec<Issue> {
    let mut issues = issue_ledger
        .issues
        .iter()
        .filter(|issue| issue.personas.contains(&persona))
        .cloned()
        .collect::<Vec<_>>();
    issues.sort_by(|left, right| {
        persona_priority(persona, left.category)
            .cmp(&persona_priority(persona, right.category))
            .then(left.severity.cmp(&right.severity))
            .then(left.id.cmp(&right.id))
    });
    issues.truncate(TOP_ISSUE_LIMIT);
    issues
}

fn unique_command_paths(paths: Vec<Vec<String>>) -> Vec<Vec<String>> {
    paths
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
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

fn evidence_for_finding(finding: &FindingArtifact, artifacts: &MeasuredArtifacts) -> Vec<String> {
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
        "existence_unconfirmed" | "help_unavailable" | "precondition_blocked" => {
            ActionCategory::Discovery
        }
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

fn persona_priority(persona: Persona, category: ActionCategory) -> u16 {
    match persona {
        Persona::Maintainer => match category {
            ActionCategory::Output => 10,
            ActionCategory::Discovery => 20,
            ActionCategory::Grammar => 30,
            ActionCategory::Recovery => 40,
            ActionCategory::Safety => 50,
            _ => 60,
        },
        Persona::Harness => match category {
            ActionCategory::Safety => 10,
            ActionCategory::Output => 20,
            ActionCategory::Discovery => 30,
            ActionCategory::Grammar => 40,
            _ => 60,
        },
        Persona::Platform => match category {
            ActionCategory::Policy => 10,
            ActionCategory::Safety => 20,
            ActionCategory::Coverage => 30,
            ActionCategory::Output => 40,
            _ => 60,
        },
        Persona::Security => match category {
            ActionCategory::Safety => 10,
            ActionCategory::Discovery => 20,
            ActionCategory::Coverage => 30,
            _ => 70,
        },
        Persona::Oss => match category {
            ActionCategory::Publishing => 10,
            ActionCategory::Output => 20,
            ActionCategory::Discovery => 30,
            ActionCategory::Coverage => 40,
            _ => 70,
        },
        Persona::Devrel => match category {
            ActionCategory::Publishing => 10,
            ActionCategory::Output => 20,
            ActionCategory::Discovery => 30,
            ActionCategory::Calibration => 40,
            _ => 70,
        },
        Persona::Research => match category {
            ActionCategory::Calibration => 10,
            ActionCategory::Coverage => 20,
            ActionCategory::Discovery => 30,
            _ => 70,
        },
    }
}

fn run_recommendations(
    persona: Persona,
    scorecard: &ScorecardArtifact,
    artifact_dir: &Path,
) -> Vec<RunRecommendation> {
    let coverage = &scorecard.coverage;
    let target = shell_arg(&scorecard.target.requested.display().to_string());
    let out = shell_arg(&artifact_dir.display().to_string());
    let mut recommendations = Vec::new();

    if !coverage.traversal_complete {
        let depth = if coverage.observed_max_depth >= coverage.max_depth {
            coverage.max_depth + 2
        } else {
            coverage.max_depth.max(8)
        };
        let probes = if coverage.budget_exhausted {
            coverage.max_probes.saturating_mul(2).max(1_000)
        } else {
            coverage.max_probes.max(1_000)
        };
        recommendations.push(RunRecommendation {
            id: "run.deepen_surface".to_owned(),
            priority: 10,
            command: format!(
                "cliare measure {target} --out {out} --profile deep --max-depth {depth} --max-probes {probes} --concurrency {} --refresh",
                coverage.concurrency_limit.max(8)
            ),
            purpose: "Expand command-surface coverage before treating this run as complete."
                .to_owned(),
            when_to_use: "Use when traversal is incomplete, depth was exhausted, or the probe frontier still has pending work."
                .to_owned(),
        });
    }

    if coverage.machine_readable_output_contracts == 0 {
        recommendations.push(RunRecommendation {
            id: "run.after_output_contracts".to_owned(),
            priority: 30,
            command: format!("cliare measure {target} --out {out} --profile standard --refresh"),
            purpose:
                "Re-measure after adding JSON or YAML output modes to read/list/show commands."
                    .to_owned(),
            when_to_use: "Use after improving machine-readable output contracts.".to_owned(),
        });
    }

    match persona {
        Persona::Platform => recommendations.push(RunRecommendation {
            id: "run.platform_guard".to_owned(),
            priority: 20,
            command: format!(
                "cliare guard {target} --baseline .cliare/baseline.scorecard.json --policy cliare.policy.json --out {out}"
            ),
            purpose: "Turn the measurement into a release gate with score and policy checks."
                .to_owned(),
            when_to_use: "Use in CI after selecting policy thresholds for the organization."
                .to_owned(),
        }),
        Persona::Security => recommendations.push(RunRecommendation {
            id: "run.security_packet".to_owned(),
            priority: 20,
            command: format!("cliare report security --out {out} --write"),
            purpose: "Persist a security-focused packet for approval review.".to_owned(),
            when_to_use: "Use whenever side effects, auth gates, or agent exposure approvals are being reviewed."
                .to_owned(),
        }),
        Persona::Harness => recommendations.push(RunRecommendation {
            id: "run.harness_json".to_owned(),
            priority: 20,
            command: format!("cliare report harness --out {out} --format json --write"),
            purpose: "Persist a machine-readable packet for tool routers and harness policy."
                .to_owned(),
            when_to_use: "Use before exposing a CLI subset to agents.".to_owned(),
        }),
        Persona::Oss | Persona::Devrel => recommendations.push(RunRecommendation {
            id: "run.publishable_standard".to_owned(),
            priority: 20,
            command: format!("cliare measure {target} --out {out} --profile standard --refresh"),
            purpose: "Refresh a publishable local scorecard before release communication."
                .to_owned(),
            when_to_use: "Use before adding badges, release notes, or public scorecard artifacts."
                .to_owned(),
        }),
        Persona::Research => recommendations.push(RunRecommendation {
            id: "run.research_deep".to_owned(),
            priority: 20,
            command: format!(
                "cliare measure {target} --out {out} --profile deep --max-depth 8 --max-probes 1000 --concurrency 8 --refresh"
            ),
            purpose: "Produce a deeper evidence set suitable for labeling and calibration review."
                .to_owned(),
            when_to_use: "Use when adding the target to a benchmark corpus or truth-set workflow."
                .to_owned(),
        }),
        Persona::Maintainer => {}
    }

    recommendations.sort_by_key(|item| item.priority);
    recommendations
}

fn notes(persona: Persona, scorecard: &ScorecardArtifact) -> Vec<OutcomeNote> {
    let mut notes = Vec::new();
    notes.push(OutcomeNote {
        level: "info",
        text: "Persona packets are projections over measured artifacts; they do not rerun the target CLI."
            .to_owned(),
    });
    notes.push(OutcomeNote {
        level: "info",
        text: "Black-box measurement reports observed evidence and uncertainty; it cannot prove that hidden command surface does not exist."
            .to_owned(),
    });
    if scorecard.score.status == "experimental_partial" {
        notes.push(OutcomeNote {
            level: "warning",
            text: "Score v0 is suitable for CI feedback and improvement tracking, not certified public ranking."
                .to_owned(),
        });
    }
    match persona {
        Persona::Security => notes.push(OutcomeNote {
            level: "warning",
            text: if scorecard.coverage.side_effect_files_total > 0 {
                "Observed side effects require review before approval; inspect evidence paths, fixture state, auth state, and traversal completeness."
            } else {
                "Absence of observed side effects is not an approval by itself; review profile, fixtures, auth state, and traversal completeness."
            }
            .to_owned(),
        }),
        Persona::Oss | Persona::Devrel => notes.push(OutcomeNote {
            level: "warning",
            text: "Public claims should distinguish local scorecards from future certified leaderboard entries."
                .to_owned(),
        }),
        Persona::Research => notes.push(OutcomeNote {
            level: "info",
            text: "Use evidence IDs, model versions, budgets, and binary fingerprint when citing or labeling this run."
                .to_owned(),
        }),
        _ => {}
    }
    notes
}

fn render_markdown(packet: &PersonaOutcomePacket) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# CLIARE {} Report", packet.persona_title)
        .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(&mut text, "{}", packet.primary_question).expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");

    render_persona_decision(&mut text, packet);
    render_ci_action_brief(&mut text, packet);
    render_persona_score_summary(&mut text, packet);
    render_persona_findings(&mut text, packet);
    render_persona_command_guidance(&mut text, packet);
    render_run_recommendations(&mut text, packet);
    render_artifact_navigation(&mut text, packet);
    render_notes(&mut text, packet);

    text
}

fn render_persona_decision(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Decision").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "{}", escape_markdown(&persona_decision(packet)))
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn render_ci_action_brief(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## CI Action Brief").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    if packet.top_issues.is_empty() {
        writeln!(
            text,
            "No persona-prioritized fixes are required by this run. Keep the scorecard and command index in CI so future drift is visible."
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "| Check | Result |").expect("writing to string cannot fail");
        writeln!(text, "|---|---|").expect("writing to string cannot fail");
        writeln!(text, "| Score | `{:.0}/100` |", packet.summary.score)
            .expect("writing to string cannot fail");
        writeln!(
            text,
            "| Command coverage | `{}/{}` runtime-confirmed |",
            packet.summary.commands_runtime_confirmed, packet.summary.commands_discovered
        )
        .expect("writing to string cannot fail");
        writeln!(
            text,
            "| Command drill-down | `{}` |",
            packet.source_artifacts.command_index.display()
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        return;
    }

    let high = packet
        .top_issues
        .iter()
        .filter(|issue| issue.severity == ActionSeverity::High)
        .count();
    let fixture_required = packet
        .top_issues
        .iter()
        .filter(|issue| issue.confidence == IssueConfidence::NeedsFixture)
        .count();
    writeln!(
        text,
        "Treat the rows below as the persona-specific CI work queue. Fix P1 first, rerun the verification command, and use `command-index.json` only when you need exact command parameters or evidence pointers."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Field | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|").expect("writing to string cannot fail");
    writeln!(text, "| Score | `{:.0}/100` |", packet.summary.score)
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Persona work queue | `{}` prioritized issue(s), `{}` high severity, `{}` need fixtures |",
        packet.top_issues.len(),
        high,
        fixture_required
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Command drill-down | `{}` |",
        packet.source_artifacts.command_index.display()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    writeln!(text, "| Priority | Do this | Where | Why | Verify |")
        .expect("writing to string cannot fail");
    writeln!(text, "|---:|---|---|---|---|").expect("writing to string cannot fail");
    for (index, issue) in packet
        .top_issues
        .iter()
        .take(PERSONA_FINDING_LIMIT)
        .enumerate()
    {
        writeln!(
            text,
            "| P{} | {} | {} | {} | `{}` |",
            index + 1,
            escape_markdown(&ci_action_text(packet.persona, issue)),
            escape_markdown(&issue_where_label(issue)),
            escape_markdown(&issue.impact),
            escape_markdown(&issue.verification.command)
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_persona_score_summary(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Score Context").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Field | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|").expect("writing to string cannot fail");
    writeln!(
        text,
        "| Target | `{}` |",
        escape_markdown(&packet.target.requested.display().to_string())
    )
    .expect("writing to string cannot fail");
    writeln!(text, "| Score | `{:.0}/100` |", packet.summary.score)
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Runtime confirmation | `{}/{}` commands ({}) |",
        packet.summary.commands_runtime_confirmed,
        packet.summary.commands_discovered,
        percent(packet.coverage.command_confirmation_rate)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Output contracts | `{}` machine-readable, `{}` parse successes |",
        packet.summary.machine_readable_output_contracts,
        packet.summary.output_mode_parse_successes
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Preconditions | `{}` blocked commands, `{}` blocked probes |",
        packet.summary.commands_precondition_blocked, packet.summary.precondition_blocked_probes
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Side effects | `{}` file changes, `{}` credential-like paths |",
        packet.summary.side_effect_files_total, packet.summary.credential_like_side_effects
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Coverage | `{}`; depth `{}/{}`, probes `{}/{}` |",
        packet.summary.traversal_stop_reason,
        packet.summary.observed_max_depth,
        packet.summary.max_depth,
        packet.summary.probes_completed,
        packet.summary.max_probes
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn ci_action_text(persona: Persona, issue: &Issue) -> String {
    match issue.confidence {
        IssueConfidence::NeedsFixture => {
            format!(
                "{} {}",
                persona_issue_action(persona, issue),
                fixture_hint(issue)
            )
        }
        IssueConfidence::Blocked => {
            format!(
                "{} Document or provision the runtime precondition before treating affected commands as available.",
                persona_issue_action(persona, issue)
            )
        }
        IssueConfidence::Inferred => {
            format!(
                "{} Confirm the candidate set before making broad CLI changes.",
                persona_issue_action(persona, issue)
            )
        }
        IssueConfidence::Observed | IssueConfidence::Advisory => {
            persona_issue_action(persona, issue).to_owned()
        }
    }
}

fn fixture_hint(issue: &Issue) -> &'static str {
    if issue
        .affected_commands
        .iter()
        .any(|command| !command.required_positionals.is_empty())
    {
        "Add a safe sample operand or fixture profile so CLIARE can validate the advertised contract."
    } else {
        "Add a safe fixture or metadata path so CLIARE can validate the advertised contract."
    }
}

fn issue_where_label(issue: &Issue) -> String {
    if issue.affected_commands.is_empty() {
        return "scorecard-level finding".to_owned();
    }
    let mut commands = issue_command_samples(issue);
    let mut labels = commands
        .drain(..)
        .take(3)
        .map(|command| format!("`{}`", command_path_label(&command.path)))
        .collect::<Vec<_>>();
    if issue.affected_commands.len() > 3 {
        labels.push(format!("... {} more", issue.affected_commands.len() - 3));
    }
    labels.join(", ")
}

fn render_persona_findings(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Priority Findings").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    if packet.top_issues.is_empty() {
        writeln!(
            text,
            "No findings are prioritized for this persona. Use `issues.json` for the full ledger."
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        return;
    }

    writeln!(
        text,
        "Start with this table, then open the matching drill-down section only when you need command samples, evidence, or verification details."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "| Priority | Severity | Category | Confidence | Affected | Evidence | Issue | Persona Action |"
    )
    .expect("writing to string cannot fail");
    writeln!(text, "|---:|---|---|---|---:|---:|---|---|").expect("writing to string cannot fail");
    for (index, issue) in packet
        .top_issues
        .iter()
        .take(PERSONA_FINDING_LIMIT)
        .enumerate()
    {
        render_persona_finding_row(text, packet.persona, issue, index + 1);
    }
    writeln!(text).expect("writing to string cannot fail");
    if packet.top_issues.len() > PERSONA_FINDING_LIMIT {
        writeln!(
            text,
            "This report shows the top {} persona findings. See `issues.json` for the remaining {} issue(s).",
            PERSONA_FINDING_LIMIT,
            packet.top_issues.len() - PERSONA_FINDING_LIMIT
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
    }

    writeln!(text, "## Finding Drill-Down").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for (index, issue) in packet
        .top_issues
        .iter()
        .take(PERSONA_FINDING_LIMIT)
        .enumerate()
    {
        render_persona_finding(text, packet.persona, issue, index + 1);
    }
}

fn render_persona_finding_row(text: &mut String, persona: Persona, issue: &Issue, priority: usize) {
    writeln!(
        text,
        "| P{} | `{}` | `{}` | `{}` | {} | {} | `{}` {} | {} |",
        priority,
        issue.severity.label(),
        issue.category.label(),
        issue.confidence.label(),
        issue.affected_commands.len(),
        issue.evidence.len(),
        escape_markdown(&issue.id),
        escape_markdown(&issue.title),
        escape_markdown(persona_issue_action(persona, issue))
    )
    .expect("writing to string cannot fail");
}

fn render_persona_finding(text: &mut String, persona: Persona, issue: &Issue, priority: usize) {
    writeln!(text, "<details>").expect("writing to string cannot fail");
    writeln!(
        text,
        "<summary>P{}: {} (`{}`)</summary>",
        priority,
        escape_markdown(&issue.title),
        escape_markdown(&issue.id)
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "### P{}: {}", priority, escape_markdown(&issue.title))
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "- Issue: `{}` (`{}`, `{}`, `{}`)",
        escape_markdown(&issue.id),
        issue.severity.label(),
        issue.category.label(),
        issue.confidence.label()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Role action: {}",
        escape_markdown(persona_issue_action(persona, issue))
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Recommended change: {}",
        escape_markdown(&issue.recommendation)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Verification: `{}`",
        escape_markdown(&issue.verification.command)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Expected improvement: {}",
        escape_markdown(&issue.verification.expected_change)
    )
    .expect("writing to string cannot fail");

    if !issue.affected_commands.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "{}:", command_section_heading(issue))
            .expect("writing to string cannot fail");
        let commands = issue_command_samples(issue);
        for command in commands.iter().take(PERSONA_COMMAND_SAMPLE_LIMIT) {
            render_issue_command_sample(text, command);
        }
        if issue.affected_commands.len() > PERSONA_COMMAND_SAMPLE_LIMIT {
            writeln!(
                text,
                "- ... {} more {} in `issues.json`.",
                issue.affected_commands.len() - PERSONA_COMMAND_SAMPLE_LIMIT,
                command_overflow_label(issue)
            )
            .expect("writing to string cannot fail");
        }
    }

    if !issue.evidence.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "{}:", evidence_section_heading(issue))
            .expect("writing to string cannot fail");
        for evidence in issue.evidence.iter().take(PERSONA_EVIDENCE_SAMPLE_LIMIT) {
            render_issue_evidence_sample(text, evidence);
        }
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "</details>").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn command_section_heading(issue: &Issue) -> &'static str {
    match issue.confidence {
        IssueConfidence::Observed => "Affected commands",
        IssueConfidence::Blocked => "Blocked command examples",
        IssueConfidence::NeedsFixture => "Fixture examples",
        IssueConfidence::Inferred if issue_has_runtime_confirmed_commands(issue) => {
            "Commands to verify"
        }
        IssueConfidence::Inferred => "Candidate examples to review",
        IssueConfidence::Advisory => "Related examples",
    }
}

fn command_overflow_label(issue: &Issue) -> &'static str {
    match issue.confidence {
        IssueConfidence::Observed => "affected command(s)",
        IssueConfidence::Blocked => "blocked command example(s)",
        IssueConfidence::NeedsFixture => "fixture example(s)",
        IssueConfidence::Inferred if issue_has_runtime_confirmed_commands(issue) => {
            "command example(s)"
        }
        IssueConfidence::Inferred => "candidate example(s)",
        IssueConfidence::Advisory => "related example(s)",
    }
}

fn evidence_section_heading(issue: &Issue) -> &'static str {
    match issue.confidence {
        IssueConfidence::Inferred | IssueConfidence::Advisory => "Evidence references",
        _ => "Evidence to open",
    }
}

fn issue_command_samples(issue: &Issue) -> Vec<&IssueCommand> {
    let mut commands = issue.affected_commands.iter().collect::<Vec<_>>();
    commands.sort_by(issue_command_sample_order);
    commands
}

fn issue_has_runtime_confirmed_commands(issue: &Issue) -> bool {
    issue
        .affected_commands
        .iter()
        .any(|command| command.state == "runtime_confirmed")
}

fn issue_command_sample_order(left: &&IssueCommand, right: &&IssueCommand) -> Ordering {
    right
        .confidence
        .unwrap_or(f64::NEG_INFINITY)
        .total_cmp(&left.confidence.unwrap_or(f64::NEG_INFINITY))
        .then_with(|| left.path.len().cmp(&right.path.len()))
        .then_with(|| left.path.cmp(&right.path))
}

fn render_issue_command_sample(text: &mut String, command: &IssueCommand) {
    let detail = if command.required_positionals.is_empty() {
        command.reason.clone()
    } else {
        let required = required_positionals_label(&command.required_positionals);
        if command.reason.contains(&required) {
            command.reason.clone()
        } else {
            format!(
                "{}; required operands {}",
                command.reason.trim_end_matches('.'),
                required
            )
        }
    };
    writeln!(
        text,
        "- `{}`: {}",
        escape_markdown(&command_path_label(&command.path)),
        escape_markdown(&detail)
    )
    .expect("writing to string cannot fail");
    for contract in command.output_contracts.iter().take(1) {
        writeln!(
            text,
            "  - Contract: `{}` via `{}` is `{}`.{}{}",
            escape_markdown(&output_mode_label(&contract.mode)),
            escape_markdown(&shell_words(&contract.argv_fragment)),
            escape_markdown(&contract.status),
            contract
                .skip_reason
                .as_ref()
                .map(|reason| format!(" {}", escape_markdown(reason)))
                .unwrap_or_default(),
            contract
                .suggested_validation
                .as_ref()
                .map(|suggestion| format!(" {}", escape_markdown(suggestion)))
                .unwrap_or_default()
        )
        .expect("writing to string cannot fail");
    }
}

fn required_positionals_label(required_positionals: &[String]) -> String {
    required_positionals
        .iter()
        .map(|name| format!("<{name}>"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_issue_evidence_sample(text: &mut String, evidence: &IssueEvidence) {
    let status = evidence
        .status
        .as_ref()
        .map(|status| format!(" `{}`", escape_markdown(status)))
        .unwrap_or_default();
    let argv = if evidence.argv.is_empty() {
        String::new()
    } else {
        format!("; `{}`", escape_markdown(&evidence.argv.join(" ")))
    };
    let detail = if evidence.detail.is_empty() {
        String::new()
    } else {
        format!(" - {}", escape_markdown(&evidence.detail))
    };
    writeln!(
        text,
        "- `{}`{}{}{}",
        escape_markdown(&evidence.reference),
        status,
        argv,
        detail
    )
    .expect("writing to string cannot fail");
}

fn render_persona_command_guidance(text: &mut String, packet: &PersonaOutcomePacket) {
    let ready = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::Ready)
        .collect::<Vec<_>>();
    let blocked = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::Blocked)
        .collect::<Vec<_>>();
    let incomplete = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::Incomplete)
        .collect::<Vec<_>>();
    let unconfirmed = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::Unconfirmed)
        .collect::<Vec<_>>();

    writeln!(text, "## Command Set Guidance").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "{}",
        escape_markdown(persona_command_guidance(packet.persona))
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "| State | Count | Persona treatment | Sample commands |"
    )
    .expect("writing to string cannot fail");
    writeln!(text, "|---|---:|---|---|").expect("writing to string cannot fail");
    render_command_guidance_row(
        text,
        "Ready",
        ready.len(),
        "Candidate for routing or promotion after local policy review.",
        &ready,
    );
    render_command_guidance_row(
        text,
        "Hold: preconditions",
        blocked.len(),
        "Requires auth, profile, fixture, or environment setup before agent use.",
        &blocked,
    );
    render_command_guidance_row(
        text,
        "Hold: incomplete evidence",
        incomplete.len(),
        "Needs better help, diagnostics, output validation, or side-effect policy.",
        &incomplete,
    );
    render_command_guidance_row(
        text,
        "Candidates only",
        unconfirmed.len(),
        "Do not expose automatically until runtime confirmation exists.",
        &unconfirmed,
    );
    writeln!(
        text,
        "| Full catalog | {} | Use the command index for command-level drill-down. | `persona-{}.json`, `command-index.json`, `shape.json` |",
        packet.command_health.len(),
        packet.persona.label()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn render_command_guidance_row(
    text: &mut String,
    state: &str,
    count: usize,
    treatment: &str,
    commands: &[&CommandHealth],
) {
    writeln!(
        text,
        "| {} | {} | {} | {} |",
        escape_markdown(state),
        count,
        escape_markdown(treatment),
        command_sample_list(commands, COMMAND_GUIDANCE_SAMPLE_LIMIT)
    )
    .expect("writing to string cannot fail");
}

fn render_run_recommendations(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Recommended Runs").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    if packet.run_recommendations.is_empty() {
        writeln!(
            text,
            "No follow-up run is required before acting on the findings above."
        )
        .expect("writing to string cannot fail");
    } else {
        writeln!(text, "| Run | Purpose | Command | Use when |")
            .expect("writing to string cannot fail");
        writeln!(text, "|---|---|---|---|").expect("writing to string cannot fail");
        for recommendation in &packet.run_recommendations {
            writeln!(
                text,
                "| `{}` | {} | `{}` | {} |",
                escape_markdown(&recommendation.id),
                escape_markdown(&recommendation.purpose),
                escape_markdown(&recommendation.command),
                escape_markdown(&use_when_text(&recommendation.when_to_use))
            )
            .expect("writing to string cannot fail");
        }
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_artifact_navigation(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Working Files").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Artifact | Use |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|").expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Full issue ledger for remediation and complete affected-command lists. |",
        packet
            .source_artifacts
            .artifact_dir
            .join("issues.json")
            .display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Human-readable issue ledger. |",
        packet
            .source_artifacts
            .artifact_dir
            .join("issues.md")
            .display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Persona packet with full command health. |",
        packet
            .source_artifacts
            .artifact_dir
            .join(format!("persona-{}.json", packet.persona.label()))
            .display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Command-centric lookup table with suitability, parameters, preconditions, output contracts, gaps, and evidence pointers. |",
        packet.source_artifacts.command_index.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Human-readable command index table. |",
        packet.source_artifacts.command_index_markdown.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Inferred command catalog, flags, gaps, and output contracts. |",
        packet.source_artifacts.shape.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Runtime evidence log used to verify claims. |",
        packet.source_artifacts.evidence.display()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Evidence summary | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---:|").expect("writing to string cannot fail");
    writeln!(
        text,
        "| Probes scheduled | {} |",
        packet.evidence_summary.probes_scheduled
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Processes completed | {} |",
        packet.evidence_summary.processes_completed
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Probe failures | {} |",
        packet.evidence_summary.probe_failures_total
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Side-effect records | {} |",
        packet.evidence_summary.side_effects_total
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn render_notes(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Notes").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for note in &packet.notes {
        writeln!(text, "- `{}` {}", note.level, escape_markdown(&note.text))
            .expect("writing to string cannot fail");
    }
}

fn persona_decision(packet: &PersonaOutcomePacket) -> String {
    let high_issues = packet
        .top_issues
        .iter()
        .filter(|issue| issue.severity == ActionSeverity::High)
        .count();
    let fixture_issues = packet
        .top_issues
        .iter()
        .filter(|issue| issue.confidence == IssueConfidence::NeedsFixture)
        .count();
    match packet.persona {
        Persona::Maintainer => {
            if packet.top_issues.is_empty() {
                "No maintainer-prioritized fixes are currently blocking the measured posture. Keep the issue ledger in CI and watch for drift.".to_owned()
            } else {
                format!(
                    "Fix the top CLI contract gaps before treating this surface as stable for agents. This report prioritizes {} issue(s), including {} high-severity item(s) and {} fixture-required output contract item(s).",
                    packet.top_issues.len(),
                    high_issues,
                    fixture_issues
                )
            }
        }
        Persona::Harness => {
            "Expose only runtime-confirmed commands with understood output and side-effect behavior. Hold blocked, unconfirmed, and fixture-required commands out of automatic routing until the listed findings are resolved.".to_owned()
        }
        Persona::Platform => {
            "Use this run as CI feedback, not a final gate, until policy thresholds and side-effect rules are explicit. Convert the top findings into guard policy before enforcing readiness across teams.".to_owned()
        }
        Persona::Security => {
            if packet.summary.side_effect_files_total > 0 || packet.summary.credential_like_side_effects > 0 {
                if packet.summary.credential_like_side_effects > 0 {
                    "Require review before approving unrestricted safe-probe use. The measurement observed filesystem side effects, including credential-like paths, that need policy treatment.".to_owned()
                } else {
                    "Require review before approving unrestricted safe-probe use. The measurement observed filesystem side effects that need policy treatment.".to_owned()
                }
            } else {
                "No credential-like side effects were observed, but approval still depends on traversal completeness, auth state, and fixture coverage.".to_owned()
            }
        }
        Persona::Oss => {
            "Publish the scorecard only with its profile, fingerprint, and caveats. Do not present the score as certified while score v0 and local calibration are still explicit.".to_owned()
        }
        Persona::Devrel => {
            "Use the report to teach concrete CLI improvements, not to market a raw score. Public claims should cite the specific measured contracts and known gaps.".to_owned()
        }
        Persona::Research => {
            "Treat this run as a labeled-candidate artifact. It is useful for replay and study only after command existence, output contracts, preconditions, and side effects are independently labeled.".to_owned()
        }
    }
}

fn persona_issue_action(persona: Persona, issue: &Issue) -> &'static str {
    match persona {
        Persona::Maintainer
            if issue.confidence == IssueConfidence::Inferred
                && issue_has_runtime_confirmed_commands(issue) =>
        {
            "Verify the measured gap on the listed commands. If the behavior is intentional, document it as a CLI contract; otherwise make the help or diagnostic path explicit."
        }
        Persona::Maintainer if issue.confidence == IssueConfidence::Inferred => {
            "Review the candidate set before changing code. Confirm true commands by improving help/catalog clarity; classify false candidates as inference noise in the ledger."
        }
        Persona::Maintainer => match issue.category {
            ActionCategory::Output => {
                "Add a safe validation path or fixture for the advertised machine-readable output contract."
            }
            ActionCategory::Discovery => {
                "Make command existence and help discoverable without relying on configured account state."
            }
            ActionCategory::Recovery => {
                "Improve diagnostics so agents can repair invalid command attempts without guessing."
            }
            ActionCategory::Safety => {
                "Keep help/version/diagnostic paths read-only, or provide a documented way to suppress expected writes."
            }
            _ => "Fix the CLI contract behind this finding and rerun the same profile.",
        },
        Persona::Harness
            if issue.confidence == IssueConfidence::Inferred
                && issue_has_runtime_confirmed_commands(issue) =>
        {
            "Treat these commands as conditional for agents until the missing help, diagnostic, or output evidence is confirmed."
        }
        Persona::Harness if issue.confidence == IssueConfidence::Inferred => {
            "Do not expose inferred candidates to agents until runtime probes confirm them or the harness provisions the missing precondition."
        }
        Persona::Harness => match issue.category {
            ActionCategory::Safety => {
                "Do not run this probe profile unattended until side-effect policy, fixtures, and allowed persistent writes are explicit."
            }
            ActionCategory::Output => {
                "Do not route agent state through this output mode until a safe fixture or parseable probe confirms it."
            }
            ActionCategory::Discovery => {
                "Treat blocked or unconfirmed commands as unavailable unless the harness provisions the required precondition."
            }
            _ => {
                "Mark affected commands as conditional in the harness catalog until the finding is resolved."
            }
        },
        Persona::Platform => {
            "Convert this finding into a CI policy decision: fail, warn, or allow with a documented exception."
        }
        Persona::Security => match issue.confidence {
            IssueConfidence::Observed => {
                "Review as observed runtime behavior with direct evidence."
            }
            IssueConfidence::Blocked => {
                "Require documented runtime preconditions before approving automated use."
            }
            IssueConfidence::NeedsFixture => {
                "Require safe fixture definitions before accepting the advertised contract."
            }
            _ => "Keep this as an uncertainty item until stronger runtime evidence exists.",
        },
        Persona::Oss => {
            "Publish this as an open remediation item with evidence and avoid claiming it is fixed until the verification command changes."
        }
        Persona::Devrel => {
            "Turn this into user-facing guidance: what behavior agents need, what the CLI currently does, and how the project will improve it."
        }
        Persona::Research => {
            "Label this finding with its confidence class and evidence references before using it for calibration."
        }
    }
}

fn persona_command_guidance(persona: Persona) -> &'static str {
    match persona {
        Persona::Harness => {
            "Use this section as the first-pass agent exposure map. Ready commands can be candidates for routing; every other bucket needs policy, fixtures, or manual review."
        }
        Persona::Security => {
            "Use this section to separate commands with confirmed safe shape from commands blocked by preconditions or incomplete evidence."
        }
        Persona::Platform => {
            "Use this section to decide which command classes should fail CI, warn, or require an exception."
        }
        Persona::Research => {
            "Use this section to preserve readiness labels for downstream analysis; do not collapse blocked, incomplete, and unconfirmed into one class."
        }
        _ => {
            "Use this section as a compact triage map. The full command catalog remains in machine-readable artifacts."
        }
    }
}

fn command_sample_list(commands: &[&CommandHealth], limit: usize) -> String {
    if commands.is_empty() {
        return "`none`".to_owned();
    }
    let mut commands = commands.to_vec();
    commands.sort_by(command_health_sample_order);
    let mut labels = commands
        .iter()
        .take(limit)
        .map(|command| format!("`{}`", escape_markdown(&command_path_label(&command.path))))
        .collect::<Vec<_>>();
    if commands.len() > limit {
        labels.push(format!("... {} more", commands.len() - limit));
    }
    labels.join(", ")
}

fn command_health_sample_order(left: &&CommandHealth, right: &&CommandHealth) -> Ordering {
    right
        .confidence
        .total_cmp(&left.confidence)
        .then_with(|| left.path.len().cmp(&right.path.len()))
        .then_with(|| left.path.cmp(&right.path))
}

fn use_when_text(value: &str) -> String {
    value
        .trim()
        .strip_prefix("Use ")
        .unwrap_or_else(|| value.trim())
        .to_owned()
}

fn percent(value: f64) -> String {
    if value.is_finite() {
        format!("{:.1}%", value * 100.0)
    } else {
        "n/a".to_owned()
    }
}

fn render_issue_ledger_markdown(ledger: &IssueLedger) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# CLIARE Issue Ledger").expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Target: `{}`",
        escape_markdown(&ledger.target.requested.display().to_string())
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Issues: `{}` (`{}` high, `{}` medium, `{}` low)",
        ledger.summary.issues_total, ledger.summary.high, ledger.summary.medium, ledger.summary.low
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Affected commands: `{}`",
        ledger.summary.affected_commands
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Fixture-required issues: `{}`",
        ledger.summary.requires_fixtures
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Precondition-blocked issues: `{}`",
        ledger.summary.blocked_by_preconditions
    )
    .expect("writing to string cannot fail");

    for issue in &ledger.issues {
        render_issue_markdown(&mut text, issue, 2);
    }

    text
}

fn render_issue_markdown(text: &mut String, issue: &Issue, heading_level: usize) {
    let heading = "#".repeat(heading_level);
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "{} {}", heading, escape_markdown(&issue.title))
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "- ID: `{}`", escape_markdown(&issue.id))
        .expect("writing to string cannot fail");
    writeln!(text, "- Severity: `{}`", issue.severity.label())
        .expect("writing to string cannot fail");
    writeln!(text, "- Category: `{}`", issue.category.label())
        .expect("writing to string cannot fail");
    writeln!(text, "- Confidence: `{}`", issue.confidence.label())
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Affected commands: `{}`",
        issue.affected_commands.len()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "**Impact:** {}", escape_markdown(&issue.impact))
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Why it matters:** {}",
        escape_markdown(&issue.why_it_matters)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Recommended fix:** {}",
        escape_markdown(&issue.recommendation)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Verify:** `{}`",
        escape_markdown(&issue.verification.command)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Expected change:** {}",
        escape_markdown(&issue.verification.expected_change)
    )
    .expect("writing to string cannot fail");

    if !issue.affected_commands.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "Affected command samples:").expect("writing to string cannot fail");
        for command in issue.affected_commands.iter().take(COMMAND_SAMPLE_LIMIT) {
            let required = if command.required_positionals.is_empty()
                || !command.output_contracts.is_empty()
            {
                String::new()
            } else {
                format!(
                    " Required operands: {}.",
                    command
                        .required_positionals
                        .iter()
                        .map(|name| format!("<{name}>"))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            };
            writeln!(
                text,
                "- `{}` ({}, confidence {}) - {}{}",
                escape_markdown(&command_path_label(&command.path)),
                escape_markdown(&command.state),
                command
                    .confidence
                    .map(|value| format!("{value:.3}"))
                    .unwrap_or_else(|| "n/a".to_owned()),
                escape_markdown(&command.reason),
                escape_markdown(&required)
            )
            .expect("writing to string cannot fail");
            for contract in &command.output_contracts {
                writeln!(
                    text,
                    "  - Output contract: `{}` via `{}`; status `{}`.{}{}",
                    escape_markdown(&output_mode_label(&contract.mode)),
                    escape_markdown(&shell_words(&contract.argv_fragment)),
                    escape_markdown(&contract.status),
                    contract
                        .skip_reason
                        .as_ref()
                        .map(|reason| format!(" {}", escape_markdown(reason)))
                        .unwrap_or_default(),
                    contract
                        .suggested_validation
                        .as_ref()
                        .map(|suggestion| format!(" {}", escape_markdown(suggestion)))
                        .unwrap_or_default()
                )
                .expect("writing to string cannot fail");
            }
        }
    }

    if !issue.evidence.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "Evidence samples:").expect("writing to string cannot fail");
        for evidence in issue.evidence.iter().take(5) {
            let argv = if evidence.argv.is_empty() {
                String::new()
            } else {
                format!(" argv `{}`", escape_markdown(&evidence.argv.join(" ")))
            };
            let status = evidence
                .status
                .as_ref()
                .map(|status| format!(" status `{}`", escape_markdown(status)))
                .unwrap_or_default();
            writeln!(
                text,
                "- `{}` {}{} - {}",
                escape_markdown(&evidence.reference),
                evidence
                    .intent
                    .as_ref()
                    .map(|intent| format!("intent `{}`", escape_markdown(intent)))
                    .unwrap_or_else(|| format!("kind `{}`", escape_markdown(&evidence.kind))),
                status,
                escape_markdown(&format!("{}{}", evidence.detail, argv))
            )
            .expect("writing to string cannot fail");
        }
    }
}

fn render_written_summary(
    packet: &PersonaOutcomePacket,
    markdown_path: Option<&PathBuf>,
    json_path: Option<&PathBuf>,
    guide_artifacts: Option<&ArtifactGuideSummary>,
) -> String {
    let mut text = String::new();
    writeln!(
        &mut text,
        "CLIARE {} outcome packet written",
        packet.persona.label()
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text, "score: {:.0}/100", packet.summary.score)
        .expect("writing to string cannot fail");
    writeln!(&mut text, "action items: {}", packet.action_items.len())
        .expect("writing to string cannot fail");
    writeln!(&mut text, "top issues: {}", packet.top_issues.len())
        .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "command health entries: {}",
        packet.command_health.len()
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "traversal complete: {}",
        packet.summary.traversal_complete
    )
    .expect("writing to string cannot fail");
    if let Some(path) = markdown_path {
        writeln!(&mut text, "markdown: {}", path.display()).expect("writing to string cannot fail");
    }
    if let Some(path) = json_path {
        writeln!(&mut text, "json: {}", path.display()).expect("writing to string cannot fail");
    }
    if let Some(path) = markdown_path {
        writeln!(
            &mut text,
            "issue ledger markdown: {}",
            path.with_file_name("issues.md").display()
        )
        .expect("writing to string cannot fail");
    }
    if let Some(path) = json_path {
        writeln!(
            &mut text,
            "issue ledger json: {}",
            path.with_file_name("issues.json").display()
        )
        .expect("writing to string cannot fail");
    }
    if let Some(artifacts) = guide_artifacts {
        writeln!(&mut text, "readme: {}", artifacts.readme_path.display())
            .expect("writing to string cannot fail");
        writeln!(
            &mut text,
            "agent guide: {}",
            artifacts.agent_skill_path.display()
        )
        .expect("writing to string cannot fail");
    }
    text
}

fn command_path_label(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(" ")
    }
}

fn shell_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn escape_markdown(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::{
        ActionCategory, ActionSeverity, Issue, IssueCommand, IssueConfidence, IssueVerification,
        Persona, ci_action_text, command_section_heading, persona_issue_action, persona_priority,
        shell_arg, use_when_text,
    };

    #[test]
    fn persona_priority_matches_primary_users() {
        assert!(
            persona_priority(Persona::Security, ActionCategory::Safety)
                < persona_priority(Persona::Security, ActionCategory::Output)
        );
        assert!(
            persona_priority(Persona::Harness, ActionCategory::Output)
                < persona_priority(Persona::Harness, ActionCategory::Recovery)
        );
    }

    #[test]
    fn shell_arguments_quote_spaces() {
        assert_eq!(shell_arg("target"), "target");
        assert_eq!(shell_arg("my target"), "'my target'");
    }

    #[test]
    fn inferred_runtime_confirmed_issues_are_commands_to_verify() {
        let issue = test_issue(
            IssueConfidence::Inferred,
            ActionCategory::Discovery,
            Some("runtime_confirmed"),
        );

        assert_eq!(command_section_heading(&issue), "Commands to verify");
        assert!(
            persona_issue_action(Persona::Maintainer, &issue)
                .starts_with("Verify the measured gap")
        );
    }

    #[test]
    fn inferred_unconfirmed_issues_are_candidate_examples() {
        let issue = test_issue(
            IssueConfidence::Inferred,
            ActionCategory::Discovery,
            Some("inferred"),
        );

        assert_eq!(
            command_section_heading(&issue),
            "Candidate examples to review"
        );
        assert!(
            persona_issue_action(Persona::Harness, &issue)
                .starts_with("Do not expose inferred candidates")
        );
    }

    #[test]
    fn harness_safety_action_is_profile_scoped() {
        let issue = test_issue(IssueConfidence::Observed, ActionCategory::Safety, None);

        let action = persona_issue_action(Persona::Harness, &issue);
        assert!(action.starts_with("Do not run this probe profile"));
        assert!(!action.contains("affected commands"));
    }

    #[test]
    fn persona_issue_markdown_is_table_row_with_drilldown() {
        let issue = test_issue(
            IssueConfidence::Observed,
            ActionCategory::Safety,
            Some("runtime_confirmed"),
        );

        let mut row = String::new();
        super::render_persona_finding_row(&mut row, Persona::Harness, &issue, 1);
        assert!(row.starts_with("| P1 | `medium` | `safety` | `observed` | 1 | 0 |"));
        assert!(row.contains("Do not run this probe profile"));

        let mut detail = String::new();
        super::render_persona_finding(&mut detail, Persona::Harness, &issue, 1);
        assert!(detail.contains("<details>"));
        assert!(detail.contains("<summary>P1: Test issue (`issue.test`)</summary>"));
        assert!(detail.contains("</details>"));
    }

    #[test]
    fn ci_action_text_makes_fixture_work_actionable() {
        let mut issue = test_issue(
            IssueConfidence::NeedsFixture,
            ActionCategory::Output,
            Some("runtime_confirmed"),
        );
        issue.affected_commands[0]
            .required_positionals
            .push("persona".to_owned());

        let action = ci_action_text(Persona::Devrel, &issue);

        assert!(action.contains("safe sample operand or fixture profile"));
        assert!(action.contains("advertised contract"));
    }

    #[test]
    fn use_when_text_removes_redundant_prefix() {
        assert_eq!(
            use_when_text("Use before exposing a CLI subset to agents."),
            "before exposing a CLI subset to agents."
        );
        assert_eq!(
            use_when_text("whenever policy changes."),
            "whenever policy changes."
        );
    }

    fn test_issue(
        confidence: IssueConfidence,
        category: ActionCategory,
        command_state: Option<&str>,
    ) -> Issue {
        Issue {
            id: "issue.test".to_owned(),
            status: "open",
            severity: ActionSeverity::Medium,
            category,
            confidence,
            title: "Test issue".to_owned(),
            impact: "impact".to_owned(),
            why_it_matters: "why".to_owned(),
            recommendation: "recommendation".to_owned(),
            verification: IssueVerification {
                command: "cliare measure test".to_owned(),
                expected_change: "expected".to_owned(),
            },
            affected_commands: command_state
                .map(|state| {
                    vec![IssueCommand {
                        path: vec!["cmd".to_owned()],
                        argv: vec!["target".to_owned(), "cmd".to_owned()],
                        state: state.to_owned(),
                        confidence: Some(0.8),
                        summary: None,
                        required_positionals: Vec::new(),
                        output_contracts: Vec::new(),
                        reason: "reason".to_owned(),
                    }]
                })
                .unwrap_or_default(),
            evidence: Vec::new(),
            personas: Vec::new(),
            score_dimensions: Vec::new(),
        }
    }
}
