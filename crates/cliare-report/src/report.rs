use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, de::IgnoredAny};
use tokio::fs;

use crate::artifact_guide;
use crate::report_evidence::EvidenceSummary;
use crate::report_markdown::{
    render_issue_ledger_markdown, render_markdown, render_written_summary,
};
use crate::report_model::*;
use cliare_cli::cli::{ReportArgs, ReportFormat};
use cliare_context as context;
use cliare_core::artifacts::{
    COMMAND_INDEX_JSON, EVIDENCE_JSONL, ISSUES_JSON, ISSUES_MD, SCORECARD_JSON, SHAPE_JSON,
    write_atomic,
};
use cliare_core::error::{CliareError, Result};
use cliare_issues::issue_disposition::IssueDispositions;
use cliare_runtime::fingerprint::TargetFingerprint;

use self::packets::ReportSelection;

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
    write_atomic(path, bytes)
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

mod actions;
mod health;
mod issue_builder;
mod issue_evidence;
mod ledger;
mod packets;
mod recommendations;
#[cfg(test)]
mod tests;
mod util;
