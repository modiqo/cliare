use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, de::IgnoredAny};
use tokio::fs;

use crate::artifact_guide;
use crate::artifacts::{
    COMMAND_INDEX_JSON, EVIDENCE_JSONL, ISSUES_JSON, ISSUES_MD, SCORECARD_JSON, SHAPE_JSON,
};
use crate::cli::{ReportArea, ReportArgs, ReportFormat};
use crate::context;
use crate::error::{CliareError, Result};
use crate::fingerprint::TargetFingerprint;
use crate::issue_disposition::IssueDispositions;
use crate::path_classification;
use crate::report_evidence::{
    EvidenceSummary, EvidenceSummaryPacket, ProcessEvidence, SideEffectRecord,
};
use crate::report_format::{output_mode_label, shell_arg, shell_words};
use crate::report_markdown::{
    render_issue_ledger_markdown, render_markdown, render_written_summary,
};
use crate::report_model::*;

pub use crate::report_model::Persona;

const PACKET_SCHEMA_VERSION: &str = "cliare.persona-outcome.v1";
const DRILLDOWN_SCHEMA_VERSION: &str = "cliare.report-drilldown.v1";
const ISSUE_LEDGER_SCHEMA_VERSION: &str = "cliare.issue-ledger.v1";
const ACTION_EVIDENCE_LIMIT: usize = 32;
const COMMAND_SAMPLE_LIMIT: usize = 5;
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
    let mut issue_ledger = IssueLedger::build(&artifact_dir, &artifacts);
    let dispositions = IssueDispositions::read_optional(&artifact_dir).await?;
    issue_ledger.apply_dispositions(&dispositions);
    let packet = PersonaOutcomePacket::build(persona, &artifact_dir, &artifacts, &issue_ledger);
    let drilldown = ReportSelection::from_args(&args)
        .map(|selection| {
            ReportDrilldownPacket::build(
                selection,
                args.with_evidence,
                persona,
                &artifact_dir,
                &artifacts,
                &issue_ledger,
            )
        })
        .transpose()?;
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
            ReportFormat::Markdown => {
                if let Some(drilldown) = &drilldown {
                    crate::report_markdown::render_drilldown_markdown(drilldown)
                } else {
                    markdown
                }
            }
            ReportFormat::Json => {
                if let Some(drilldown) = &drilldown {
                    format!(
                        "{}\n",
                        serde_json::to_string_pretty(drilldown)
                            .map_err(CliareError::SerializePersonaOutcome)?
                    )
                } else {
                    format!("{json}\n")
                }
            }
            ReportFormat::Bundle => {
                if let Some(drilldown) = &drilldown {
                    render_bundle(
                        &crate::report_markdown::render_drilldown_markdown(drilldown),
                        drilldown,
                    )?
                } else {
                    render_bundle(&markdown, &packet)?
                }
            }
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
    let mut issue_ledger = IssueLedger::build(out_dir, &artifacts);
    let dispositions = IssueDispositions::read_optional(out_dir).await?;
    issue_ledger.apply_dispositions(&dispositions);
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

fn render_bundle<T>(markdown: &str, value: &T) -> Result<String>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string_pretty(value).map_err(CliareError::SerializePersonaOutcome)?;
    Ok(format!("{markdown}\n## JSON\n\n```json\n{json}\n```\n"))
}

impl PersonaOutcomePacket {
    fn build(
        persona: Persona,
        artifact_dir: &Path,
        artifacts: &MeasuredArtifacts,
        issue_ledger: &IssueLedger,
    ) -> Self {
        let command_health = command_health(&artifacts.command_index);
        let summary = OutcomeSummary::from_artifacts(artifacts, command_health.len());
        let action_items = action_items(persona, artifacts);
        let run_recommendations = run_recommendations(persona, &artifacts.scorecard, artifact_dir);
        let notes = notes(persona, &artifacts.scorecard);
        let top_issues = top_issues_for_persona(persona, issue_ledger);
        let reviewed_issues = reviewed_issues_for_persona(persona, issue_ledger);

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
            reviewed_issues,
            action_items,
            command_health,
            score: ScoreSection::from(&artifacts.scorecard),
            coverage: CoverageSection::from(&artifacts.scorecard.coverage),
            evidence_summary: EvidenceSummaryPacket::from(&artifacts.evidence),
            notes,
        }
    }
}

impl ReportDrilldownPacket {
    fn build(
        selection: ReportSelection,
        with_evidence: bool,
        persona: Persona,
        artifact_dir: &Path,
        artifacts: &MeasuredArtifacts,
        issue_ledger: &IssueLedger,
    ) -> Result<Self> {
        let command_health = command_health(&artifacts.command_index);
        let summary = OutcomeSummary::from_artifacts(artifacts, command_health.len());
        let filter = selection.filter();
        let mut issues = selection.select(persona, &issue_ledger.issues);
        if issues.is_empty() {
            return Err(CliareError::ReportFilterNoMatch {
                message: format!(
                    "no {} report issues matched {} `{}`",
                    persona.label(),
                    filter.kind.label(),
                    filter.value
                ),
            });
        }
        if !with_evidence {
            for issue in &mut issues {
                issue.evidence.clear();
            }
        }

        Ok(Self {
            schema_version: DRILLDOWN_SCHEMA_VERSION,
            persona,
            persona_title: persona.title(),
            primary_question: persona.primary_question(),
            target: artifacts.scorecard.target.clone(),
            source_artifacts: SourceArtifacts::new(artifact_dir),
            summary,
            filter,
            evidence_included: with_evidence,
            issues,
        })
    }
}

#[derive(Debug, Clone)]
enum ReportSelection {
    Area(AgentReadinessArea),
    Issue(String),
}

impl ReportSelection {
    fn from_args(args: &ReportArgs) -> Option<Self> {
        args.area
            .map(|area| Self::Area(AgentReadinessArea::from(area)))
            .or_else(|| args.issue.clone().map(Self::Issue))
    }

    fn filter(&self) -> ReportDrilldownFilter {
        match self {
            Self::Area(area) => ReportDrilldownFilter {
                kind: ReportDrilldownFilterKind::Area,
                value: area.slug().to_owned(),
            },
            Self::Issue(issue) => ReportDrilldownFilter {
                kind: ReportDrilldownFilterKind::Issue,
                value: issue.clone(),
            },
        }
    }

    fn select(&self, persona: Persona, issues: &[Issue]) -> Vec<Issue> {
        issues
            .iter()
            .filter(|issue| match self {
                Self::Area(area) => {
                    issue.personas.contains(&persona) && issue.agent_readiness_area == *area
                }
                Self::Issue(id) => issue.id == *id,
            })
            .cloned()
            .collect()
    }
}

impl From<ReportArea> for AgentReadinessArea {
    fn from(value: ReportArea) -> Self {
        match value {
            ReportArea::OutputContracts => Self::OutputContracts,
            ReportArea::Preconditions => Self::Preconditions,
            ReportArea::CommandDiscovery => Self::CommandDiscovery,
            ReportArea::HelpCoverage => Self::HelpCoverage,
            ReportArea::Compatibility => Self::Compatibility,
            ReportArea::Diagnostics => Self::Diagnostics,
            ReportArea::Execution => Self::Execution,
            ReportArea::Safety => Self::Safety,
            ReportArea::Coverage => Self::Coverage,
            ReportArea::Policy => Self::Policy,
            ReportArea::Publishing => Self::Publishing,
            ReportArea::Calibration => Self::Calibration,
        }
    }
}

impl ReportDrilldownFilterKind {
    fn label(self) -> &'static str {
        match self {
            Self::Area => "area",
            Self::Issue => "issue",
        }
    }
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
            commands_ready: artifacts.command_index.summary.ready,
            commands_conditional: artifacts.command_index.summary.conditional,
            commands_needs_fixture: artifacts.command_index.summary.needs_fixture,
            commands_blocked: artifacts.command_index.summary.blocked,
            commands_candidate: artifacts.command_index.summary.candidate,
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

impl ActionSeverity {
    fn from_scorecard(value: &str) -> Self {
        match value {
            "high" => Self::High,
            "medium" => Self::Medium,
            _ => Self::Low,
        }
    }
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
                    artifact_dir,
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

    fn apply_dispositions(&mut self, dispositions: &IssueDispositions) {
        let disposition_by_id = dispositions.by_issue_id();
        for issue in &mut self.issues {
            if let Some(disposition) = disposition_by_id.get(issue.id.as_str()) {
                issue.disposition = Some((*disposition).clone());
            }
        }
        self.summary = IssueLedgerSummary::from_issues(&self.issues);
    }
}

impl IssueLedgerSummary {
    fn from_issues(issues: &[Issue]) -> Self {
        let mut affected_commands = BTreeSet::<Vec<String>>::new();
        let mut high = 0_usize;
        let mut medium = 0_usize;
        let mut low = 0_usize;
        let mut requires_fixtures = 0_usize;
        let mut blocked_by_preconditions = 0_usize;
        let mut dispositioned = 0_usize;
        let mut action_required = 0_usize;
        let mut reviewed_decisions = 0_usize;

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
            if issue_action_required(issue) {
                action_required += 1;
            }
            if let Some(disposition) = &issue.disposition {
                dispositioned += 1;
                if !disposition.status.is_action_required() {
                    reviewed_decisions += 1;
                }
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
            dispositioned,
            action_required,
            reviewed_decisions,
        }
    }
}

struct MeasuredArtifacts {
    scorecard: ScorecardArtifact,
    shape: ShapeArtifact,
    command_index: CommandIndexArtifact,
    evidence: EvidenceSummary,
}

impl MeasuredArtifacts {
    async fn read(out_dir: &Path) -> Result<Self> {
        let scorecard = read_json::<ScorecardArtifact>(&out_dir.join(SCORECARD_JSON)).await?;
        let shape = read_json::<ShapeArtifact>(&out_dir.join(SHAPE_JSON)).await?;
        let command_index =
            read_json::<CommandIndexArtifact>(&out_dir.join(COMMAND_INDEX_JSON)).await?;
        let evidence = EvidenceSummary::read(&out_dir.join(EVIDENCE_JSONL)).await?;
        Ok(Self {
            scorecard,
            shape,
            command_index,
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
    output_contracts: Vec<ShapeOutputContract>,
    gaps: Vec<ShapeGap>,
}

#[derive(Debug, Deserialize)]
struct ShapeCommand {
    path: Vec<String>,
    argv: Vec<String>,
    summary: Option<String>,
    positionals: Vec<ShapePositionalArgument>,
    confidence: f64,
    runtime_state: String,
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
struct ShapeOutputContract {
    command_path: Vec<String>,
    mode: String,
    flag_name: String,
    argv_fragment: Vec<String>,
    probed: bool,
    parse_success: bool,
    precondition_blocked: bool,
    observed_kind: Option<String>,
    diagnostic: Option<String>,
    #[serde(default)]
    help_behavior: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ShapeGap {
    kind: String,
    command_path: Vec<String>,
    reason: String,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CommandIndexArtifact {
    summary: CommandIndexSummaryArtifact,
    commands: Vec<CommandIndexCommand>,
}

#[derive(Debug, Deserialize)]
struct CommandIndexSummaryArtifact {
    ready: usize,
    conditional: usize,
    needs_fixture: usize,
    blocked: usize,
    candidate: usize,
}

#[derive(Debug, Deserialize)]
struct CommandIndexCommand {
    id: String,
    path: Vec<String>,
    argv: Vec<String>,
    summary: Option<String>,
    runtime_state: String,
    agent_suitability: String,
    #[serde(default)]
    suitability_reasons: Vec<String>,
    confidence: f64,
    parameters: CommandIndexParameters,
    #[serde(default)]
    preconditions: Vec<String>,
    #[serde(default)]
    output_contracts: Vec<CommandIndexOutputContract>,
    #[serde(default)]
    gaps: Vec<CommandIndexGap>,
    evidence: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CommandIndexParameters {
    #[serde(default)]
    flags: Vec<IgnoredAny>,
}

#[derive(Debug, Deserialize)]
struct CommandIndexOutputContract {
    mode: String,
    flag_name: String,
    argv_fragment: Vec<String>,
    status: String,
    #[serde(default)]
    preconditions: Vec<String>,
    observed_kind: Option<String>,
    diagnostic: Option<String>,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CommandIndexGap {
    kind: String,
    reason: String,
    evidence: Vec<String>,
}

fn command_health(command_index: &CommandIndexArtifact) -> Vec<CommandHealth> {
    command_index
        .commands
        .iter()
        .map(|command| {
            let output_contracts = command
                .output_contracts
                .iter()
                .map(command_health_output_contract)
                .collect::<Vec<_>>();
            CommandHealth {
                id: command.id.clone(),
                path: command.path.clone(),
                argv: command.argv.clone(),
                summary: command.summary.clone(),
                confidence: command.confidence,
                runtime_state: command.runtime_state.clone(),
                readiness_state: readiness_state(&command.agent_suitability),
                suitability_reasons: command.suitability_reasons.clone(),
                preconditions: command.preconditions.clone(),
                flags_discovered: command.parameters.flags.len(),
                output_contracts,
                gaps: command
                    .gaps
                    .iter()
                    .map(|gap| CommandGap {
                        kind: gap.kind.clone(),
                        reason: gap.reason.clone(),
                        evidence: gap.evidence.clone(),
                    })
                    .collect(),
                evidence: command.evidence.clone(),
            }
        })
        .collect()
}

fn command_health_output_contract(contract: &CommandIndexOutputContract) -> CommandOutputContract {
    CommandOutputContract {
        mode: contract.mode.clone(),
        flag_name: contract.flag_name.clone(),
        argv_fragment: contract.argv_fragment.clone(),
        status: contract.status.clone(),
        preconditions: contract.preconditions.clone(),
        advertised: true,
        probed: contract.status != "unprobed",
        parse_success: contract.status == "parse_success",
        precondition_blocked: contract.status == "precondition_blocked",
        observed_kind: contract.observed_kind.clone(),
        diagnostic: contract.diagnostic.clone(),
        help_probed: false,
        help_behavior: None,
        help_parse_success: false,
        help_diagnostic: None,
        evidence: contract.evidence.clone(),
    }
}

fn readiness_state(agent_suitability: &str) -> CommandReadinessState {
    match agent_suitability {
        "ready" => CommandReadinessState::Ready,
        "conditional" => CommandReadinessState::Conditional,
        "needs_fixture" => CommandReadinessState::NeedsFixture,
        "blocked" => CommandReadinessState::Blocked,
        _ => CommandReadinessState::Candidate,
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
    artifact_dir: &Path,
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
    let verification = issue_verification(&item, confidence, artifact_dir, artifacts);

    Issue {
        id: item.id.clone().replace("shape.gap.", "issue."),
        status: "open",
        severity: item.severity,
        category: item.category,
        agent_readiness_area: agent_readiness_area(&item),
        confidence,
        title: item.title,
        impact: issue_impact(item.category, confidence).to_owned(),
        why_it_matters: issue_why_it_matters(item.category).to_owned(),
        recommendation: item.recommendation,
        verification,
        affected_commands,
        evidence,
        disposition: None,
        personas: personas_for_issue(item.category, confidence),
        score_dimensions,
    }
}

fn agent_readiness_area(item: &ActionItem) -> AgentReadinessArea {
    if item.id.contains("output_mode") {
        return AgentReadinessArea::OutputContracts;
    }
    if item.id.contains("precondition") || item.id.contains("auth_required") {
        return AgentReadinessArea::Preconditions;
    }
    if item.id.contains("existence_unconfirmed") {
        return AgentReadinessArea::CommandDiscovery;
    }
    if item.id.contains("alternate_help_form_unavailable") {
        return AgentReadinessArea::Compatibility;
    }
    if item.id.contains("help_unavailable") {
        return AgentReadinessArea::HelpCoverage;
    }
    if item.id.contains("diagnostics_unknown") {
        return AgentReadinessArea::Diagnostics;
    }

    match item.category {
        ActionCategory::Discovery => AgentReadinessArea::CommandDiscovery,
        ActionCategory::Grammar | ActionCategory::Recovery => AgentReadinessArea::Diagnostics,
        ActionCategory::Execution => AgentReadinessArea::Execution,
        ActionCategory::Output => AgentReadinessArea::OutputContracts,
        ActionCategory::Safety => AgentReadinessArea::Safety,
        ActionCategory::Coverage => AgentReadinessArea::Coverage,
        ActionCategory::Policy => AgentReadinessArea::Policy,
        ActionCategory::Publishing => AgentReadinessArea::Publishing,
        ActionCategory::Calibration => AgentReadinessArea::Calibration,
    }
}

fn issue_confidence(item: &ActionItem) -> IssueConfidence {
    if item.id.contains("alternate_help_form_unavailable") {
        return IssueConfidence::Advisory;
    }
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
    artifact_dir: &Path,
    artifacts: &MeasuredArtifacts,
) -> IssueVerification {
    let target = shell_arg(&artifacts.scorecard.target.requested.display().to_string());
    let out = shell_arg(&artifact_dir.display().to_string());
    let command = format!("cliare measure {target} --out {out} --profile deep --refresh");
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
        (ActionCategory::Discovery, IssueConfidence::Advisory) => {
            "Optional compatibility can improve agent navigation, but canonical direct help remains the routing contract."
        }
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
        .filter(|issue| issue.personas.contains(&persona) && issue_action_required(issue))
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

fn reviewed_issues_for_persona(persona: Persona, issue_ledger: &IssueLedger) -> Vec<Issue> {
    let mut issues = issue_ledger
        .issues
        .iter()
        .filter(|issue| issue.personas.contains(&persona) && !issue_action_required(issue))
        .cloned()
        .collect::<Vec<_>>();
    issues.sort_by(|left, right| {
        left.disposition
            .as_ref()
            .map(|entry| entry.status)
            .cmp(&right.disposition.as_ref().map(|entry| entry.status))
            .then(left.id.cmp(&right.id))
    });
    issues.truncate(TOP_ISSUE_LIMIT);
    issues
}

fn issue_action_required(issue: &Issue) -> bool {
    issue
        .disposition
        .as_ref()
        .is_none_or(|disposition| disposition.status.is_action_required())
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
#[cfg(test)]
mod tests {
    use super::{
        ActionCategory, ActionSeverity, AgentReadinessArea, CommandIndexArtifact,
        CommandIndexCommand, CommandIndexGap, CommandIndexParameters, CommandIndexSummaryArtifact,
        CommandReadinessState, Issue, IssueConfidence, IssueVerification, Persona, ReportSelection,
        command_health, persona_priority,
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
    fn command_health_uses_command_index_readiness() {
        let index = CommandIndexArtifact {
            summary: CommandIndexSummaryArtifact {
                ready: 1,
                conditional: 0,
                needs_fixture: 0,
                blocked: 0,
                candidate: 0,
            },
            commands: vec![CommandIndexCommand {
                id: "rote.flow.list".to_owned(),
                path: vec!["flow".to_owned(), "list".to_owned()],
                argv: vec!["rote".to_owned(), "flow".to_owned(), "list".to_owned()],
                summary: Some("List flows".to_owned()),
                runtime_state: "runtime_confirmed".to_owned(),
                agent_suitability: "ready".to_owned(),
                suitability_reasons: vec![
                    "runtime-confirmed with parseable machine-readable output".to_owned(),
                ],
                confidence: 0.99,
                parameters: CommandIndexParameters::default(),
                preconditions: Vec::new(),
                output_contracts: Vec::new(),
                gaps: vec![CommandIndexGap {
                    kind: "alternate_help_form_unavailable".to_owned(),
                    reason: "optional `help <command path>` probe did not resolve this command"
                        .to_owned(),
                    evidence: vec!["e_000345".to_owned()],
                }],
                evidence: vec!["e_000889".to_owned()],
            }],
        };

        let health = command_health(&index);

        assert_eq!(health.len(), 1);
        assert_eq!(health[0].readiness_state, CommandReadinessState::Ready);
        assert_eq!(health[0].gaps[0].kind, "alternate_help_form_unavailable");
        assert_eq!(
            health[0].suitability_reasons,
            ["runtime-confirmed with parseable machine-readable output"]
        );
    }

    #[test]
    fn report_selection_area_is_persona_scoped() {
        let issues = vec![
            issue(
                "issue.output_mode_unprobed",
                AgentReadinessArea::OutputContracts,
                &[Persona::Maintainer],
            ),
            issue(
                "issue.security_side_effect",
                AgentReadinessArea::Safety,
                &[Persona::Security],
            ),
        ];

        let selected = ReportSelection::Area(AgentReadinessArea::OutputContracts)
            .select(Persona::Maintainer, &issues);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "issue.output_mode_unprobed");
    }

    #[test]
    fn report_selection_issue_id_is_exact() {
        let issues = vec![
            issue(
                "issue.output_mode_unprobed",
                AgentReadinessArea::OutputContracts,
                &[Persona::Maintainer],
            ),
            issue(
                "issue.output_mode_parse_failed",
                AgentReadinessArea::OutputContracts,
                &[Persona::Maintainer],
            ),
        ];

        let selected = ReportSelection::Issue("issue.output_mode_parse_failed".to_owned())
            .select(Persona::Maintainer, &issues);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "issue.output_mode_parse_failed");
    }

    fn issue(id: &str, area: AgentReadinessArea, personas: &[Persona]) -> Issue {
        Issue {
            id: id.to_owned(),
            status: "open",
            severity: ActionSeverity::Medium,
            category: ActionCategory::Output,
            agent_readiness_area: area,
            confidence: IssueConfidence::Observed,
            title: "Issue title".to_owned(),
            impact: "impact".to_owned(),
            why_it_matters: "why".to_owned(),
            recommendation: "recommendation".to_owned(),
            verification: IssueVerification {
                command: "cliare measure target".to_owned(),
                expected_change: "expected".to_owned(),
            },
            affected_commands: Vec::new(),
            evidence: Vec::new(),
            disposition: None,
            personas: personas.to_vec(),
            score_dimensions: Vec::new(),
        }
    }
}
