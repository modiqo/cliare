use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::report_evidence::EvidenceSummaryPacket;
use cliare_cli::cli::ReportPersona;
use cliare_core::artifacts::MeasurementArtifactPaths;
use cliare_issues::issue_disposition::IssueDisposition;
use cliare_runtime::fingerprint::TargetFingerprint;

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

    pub(crate) fn title(self) -> &'static str {
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

    pub(crate) fn primary_question(self) -> &'static str {
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
    pub(crate) schema_version: &'static str,
    pub(crate) persona: Persona,
    pub(crate) persona_title: &'static str,
    pub(crate) primary_question: &'static str,
    pub(crate) target: TargetFingerprint,
    pub(crate) source_artifacts: SourceArtifacts,
    pub(crate) summary: OutcomeSummary,
    pub(crate) run_recommendations: Vec<RunRecommendation>,
    pub(crate) top_issues: Vec<Issue>,
    pub(crate) reviewed_issues: Vec<Issue>,
    pub(crate) action_items: Vec<ActionItem>,
    pub(crate) command_health: Vec<CommandHealth>,
    pub(crate) agent_navigation: AgentNavigationSection,
    pub(crate) score: ScoreSection,
    pub(crate) coverage: CoverageSection,
    pub(crate) evidence_summary: EvidenceSummaryPacket,
    pub(crate) notes: Vec<OutcomeNote>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReportDrilldownPacket {
    pub(crate) schema_version: &'static str,
    pub(crate) persona: Persona,
    pub(crate) persona_title: &'static str,
    pub(crate) primary_question: &'static str,
    pub(crate) target: TargetFingerprint,
    pub(crate) source_artifacts: SourceArtifacts,
    pub(crate) summary: OutcomeSummary,
    pub(crate) filter: ReportDrilldownFilter,
    pub(crate) evidence_included: bool,
    pub(crate) issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReportDrilldownFilter {
    pub(crate) kind: ReportDrilldownFilterKind,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReportDrilldownFilterKind {
    Area,
    Issue,
}

#[derive(Debug, Serialize)]
pub(crate) struct SourceArtifacts {
    pub(crate) artifact_dir: PathBuf,
    pub(crate) evidence: PathBuf,
    pub(crate) shape: PathBuf,
    pub(crate) command_index: PathBuf,
    pub(crate) command_index_markdown: PathBuf,
    pub(crate) scorecard: PathBuf,
}

impl SourceArtifacts {
    pub(crate) fn new(artifact_dir: &Path) -> Self {
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
pub(crate) struct OutcomeSummary {
    pub(crate) score: f64,
    pub(crate) score_model: String,
    pub(crate) score_status: String,
    pub(crate) measured_weight: f64,
    pub(crate) max_weight: f64,
    pub(crate) findings: usize,
    pub(crate) shape_gaps: usize,
    pub(crate) commands_discovered: usize,
    pub(crate) commands_runtime_confirmed: usize,
    pub(crate) commands_precondition_blocked: usize,
    pub(crate) command_health_entries: usize,
    pub(crate) commands_ready: usize,
    pub(crate) commands_conditional: usize,
    pub(crate) commands_needs_fixture: usize,
    pub(crate) commands_blocked: usize,
    pub(crate) commands_candidate: usize,
    pub(crate) output_contracts_discovered: usize,
    pub(crate) machine_readable_output_contracts: usize,
    pub(crate) output_mode_parse_successes: usize,
    pub(crate) side_effect_files_total: usize,
    pub(crate) credential_like_side_effects: usize,
    pub(crate) precondition_blocked_probes: usize,
    pub(crate) auth_required_probes: usize,
    pub(crate) local_context_required_probes: usize,
    pub(crate) fixture_required_probes: usize,
    pub(crate) observed_max_depth: usize,
    pub(crate) max_depth: usize,
    pub(crate) max_probes: usize,
    pub(crate) probes_completed: usize,
    pub(crate) frontier_remaining: usize,
    pub(crate) budget_exhausted: bool,
    pub(crate) traversal_stop_reason: String,
    pub(crate) traversal_complete: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentNavigationSection {
    pub(crate) status: String,
    pub(crate) dimensions: BTreeMap<String, AgentNavigationMetricPacket>,
    pub(crate) limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentNavigationMetricPacket {
    pub(crate) score: Option<f64>,
    pub(crate) numerator: usize,
    pub(crate) denominator: usize,
    pub(crate) status: String,
    pub(crate) rationale: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ScoreSection {
    pub(crate) total: f64,
    pub(crate) measured_weight: f64,
    pub(crate) max_weight: f64,
    pub(crate) model: String,
    pub(crate) status: String,
    pub(crate) subscores: BTreeMap<String, ScoreSubscorePacket>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ScoreSubscorePacket {
    pub(crate) score: Option<f64>,
    pub(crate) weight: f64,
    pub(crate) status: String,
    pub(crate) rationale: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CoverageSection {
    pub(crate) commands_discovered: usize,
    pub(crate) commands_runtime_confirmed: usize,
    pub(crate) commands_precondition_blocked: usize,
    pub(crate) command_confirmation_rate: f64,
    pub(crate) flags_discovered: usize,
    pub(crate) output_contracts_discovered: usize,
    pub(crate) machine_readable_output_contracts: usize,
    pub(crate) output_mode_probes_completed: usize,
    pub(crate) output_mode_parse_successes: usize,
    pub(crate) output_mode_precondition_blocked: usize,
    pub(crate) side_effect_files_created: usize,
    pub(crate) side_effect_files_modified: usize,
    pub(crate) side_effect_files_deleted: usize,
    pub(crate) side_effect_files_total: usize,
    pub(crate) side_effect_probe_count: usize,
    pub(crate) credential_like_side_effects: usize,
    pub(crate) avg_command_confidence: f64,
    pub(crate) avg_flag_confidence: f64,
    pub(crate) observed_max_depth: usize,
    pub(crate) traversal_profile: String,
    pub(crate) max_depth: usize,
    pub(crate) max_probes: usize,
    pub(crate) min_expected_value: u16,
    pub(crate) concurrency_limit: usize,
    pub(crate) traversal_rounds: usize,
    pub(crate) probes_scheduled: usize,
    pub(crate) probes_completed: usize,
    pub(crate) probes_cancelled: usize,
    pub(crate) probes_timed_out: usize,
    pub(crate) probes_failed_to_spawn: usize,
    pub(crate) precondition_blocked_probes: usize,
    pub(crate) auth_required_probes: usize,
    pub(crate) local_context_required_probes: usize,
    pub(crate) fixture_required_probes: usize,
    pub(crate) frontier_remaining: usize,
    pub(crate) highest_pending_expected_value: Option<u16>,
    pub(crate) candidates_skipped_by_depth: usize,
    pub(crate) candidates_skipped_by_convergence: usize,
    pub(crate) probes_skipped_by_budget: usize,
    pub(crate) budget_exhausted: bool,
    pub(crate) traversal_stop_reason: String,
    pub(crate) traversal_complete: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunRecommendation {
    pub(crate) id: String,
    pub(crate) priority: u16,
    pub(crate) command: String,
    pub(crate) purpose: String,
    pub(crate) when_to_use: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActionSeverity {
    High,
    Medium,
    Low,
}

impl ActionSeverity {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActionCategory {
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
    pub(crate) fn label(self) -> &'static str {
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
pub(crate) struct ActionItem {
    pub(crate) id: String,
    pub(crate) severity: ActionSeverity,
    pub(crate) category: ActionCategory,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) recommendation: String,
    pub(crate) affected_count: usize,
    pub(crate) sample_command_paths: Vec<Vec<String>>,
    pub(crate) command_paths: Vec<Vec<String>>,
    pub(crate) evidence_count: usize,
    pub(crate) evidence: Vec<String>,
    pub(crate) dimension: Option<String>,
    pub(crate) persona_priority: u16,
}

#[derive(Debug, Serialize)]
pub(crate) struct IssueLedger {
    pub(crate) schema_version: &'static str,
    pub(crate) target: TargetFingerprint,
    pub(crate) source_artifacts: SourceArtifacts,
    pub(crate) summary: IssueLedgerSummary,
    pub(crate) issues: Vec<Issue>,
}

#[derive(Debug, Serialize)]
pub(crate) struct IssueLedgerSummary {
    pub(crate) issues_total: usize,
    pub(crate) high: usize,
    pub(crate) medium: usize,
    pub(crate) low: usize,
    pub(crate) affected_commands: usize,
    pub(crate) requires_fixtures: usize,
    pub(crate) blocked_by_preconditions: usize,
    pub(crate) dispositioned: usize,
    pub(crate) action_required: usize,
    pub(crate) reviewed_decisions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Issue {
    pub(crate) id: String,
    pub(crate) status: &'static str,
    pub(crate) severity: ActionSeverity,
    pub(crate) category: ActionCategory,
    pub(crate) agent_readiness_area: AgentReadinessArea,
    pub(crate) confidence: IssueConfidence,
    pub(crate) title: String,
    pub(crate) impact: String,
    pub(crate) why_it_matters: String,
    pub(crate) recommendation: String,
    pub(crate) verification: IssueVerification,
    pub(crate) affected_commands: Vec<IssueCommand>,
    pub(crate) evidence: Vec<IssueEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) disposition: Option<IssueDisposition>,
    pub(crate) personas: Vec<Persona>,
    pub(crate) score_dimensions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentReadinessArea {
    OutputContracts,
    Preconditions,
    CommandDiscovery,
    HelpCoverage,
    Compatibility,
    Diagnostics,
    Execution,
    Safety,
    Coverage,
    Policy,
    Publishing,
    Calibration,
}

impl AgentReadinessArea {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OutputContracts => "Output Contracts",
            Self::Preconditions => "Preconditions",
            Self::CommandDiscovery => "Command Discovery",
            Self::HelpCoverage => "Help Coverage",
            Self::Compatibility => "Compatibility",
            Self::Diagnostics => "Diagnostics",
            Self::Execution => "Execution",
            Self::Safety => "Safety",
            Self::Coverage => "Coverage",
            Self::Policy => "Policy",
            Self::Publishing => "Publishing",
            Self::Calibration => "Calibration",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::OutputContracts => "output-contracts",
            Self::Preconditions => "preconditions",
            Self::CommandDiscovery => "command-discovery",
            Self::HelpCoverage => "help-coverage",
            Self::Compatibility => "compatibility",
            Self::Diagnostics => "diagnostics",
            Self::Execution => "execution",
            Self::Safety => "safety",
            Self::Coverage => "coverage",
            Self::Policy => "policy",
            Self::Publishing => "publishing",
            Self::Calibration => "calibration",
        }
    }

    pub(crate) fn agent_impact(self) -> &'static str {
        match self {
            Self::OutputContracts => "Agents cannot reliably read command results.",
            Self::Preconditions => "Agents cannot distinguish missing setup from missing commands.",
            Self::CommandDiscovery => "Agents may route to commands that are not real.",
            Self::HelpCoverage => "Agents must guess syntax or recover through retries.",
            Self::Compatibility => "Navigation is less convenient, but routing can still work.",
            Self::Diagnostics => "Agents get weaker repair signals after bad invocations.",
            Self::Execution => "Agents need predictable behavior between probes and real tasks.",
            Self::Safety => "Agents may trigger unexpected persistent state changes.",
            Self::Coverage => "Agents and reviewers have incomplete evidence.",
            Self::Policy => "Teams cannot turn evidence into a repeatable gate.",
            Self::Publishing => "Public agent-readiness claims may overstate the evidence.",
            Self::Calibration => "Benchmark reuse is weak without labels and provenance.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IssueConfidence {
    Observed,
    Blocked,
    Inferred,
    NeedsFixture,
    Advisory,
}

impl IssueConfidence {
    pub(crate) fn label(self) -> &'static str {
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
pub(crate) struct IssueVerification {
    pub(crate) command: String,
    pub(crate) expected_change: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct IssueCommand {
    pub(crate) path: Vec<String>,
    pub(crate) argv: Vec<String>,
    pub(crate) state: String,
    pub(crate) confidence: Option<f64>,
    pub(crate) summary: Option<String>,
    pub(crate) required_positionals: Vec<String>,
    pub(crate) output_contracts: Vec<IssueOutputContract>,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct IssueOutputContract {
    pub(crate) mode: String,
    pub(crate) flag_name: String,
    pub(crate) argv_fragment: Vec<String>,
    pub(crate) status: String,
    pub(crate) probed: bool,
    pub(crate) parse_success: bool,
    pub(crate) precondition_blocked: bool,
    pub(crate) diagnostic: Option<String>,
    pub(crate) help_behavior: Option<String>,
    pub(crate) skip_reason: Option<String>,
    pub(crate) suggested_validation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct IssueEvidence {
    pub(crate) kind: String,
    pub(crate) reference: String,
    pub(crate) detail: String,
    pub(crate) probe_id: Option<String>,
    pub(crate) intent: Option<String>,
    pub(crate) scope: String,
    pub(crate) argv: Vec<String>,
    pub(crate) status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) interpretation: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) side_effects: Vec<IssueSideEffect>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct IssueSideEffect {
    pub(crate) operation: String,
    pub(crate) region: String,
    pub(crate) path: String,
    pub(crate) credential_like: bool,
    pub(crate) size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CommandReadinessState {
    Ready,
    Conditional,
    NeedsFixture,
    Blocked,
    Candidate,
}

#[derive(Debug, Serialize)]
pub(crate) struct CommandHealth {
    pub(crate) id: String,
    pub(crate) path: Vec<String>,
    pub(crate) argv: Vec<String>,
    pub(crate) summary: Option<String>,
    pub(crate) confidence: f64,
    pub(crate) runtime_state: String,
    pub(crate) readiness_state: CommandReadinessState,
    pub(crate) suitability_reasons: Vec<String>,
    pub(crate) preconditions: Vec<String>,
    pub(crate) flags_discovered: usize,
    pub(crate) output_contracts: Vec<CommandOutputContract>,
    pub(crate) gaps: Vec<CommandGap>,
    pub(crate) evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CommandOutputContract {
    pub(crate) mode: String,
    pub(crate) flag_name: String,
    pub(crate) argv_fragment: Vec<String>,
    pub(crate) status: String,
    pub(crate) preconditions: Vec<String>,
    pub(crate) advertised: bool,
    pub(crate) probed: bool,
    pub(crate) parse_success: bool,
    pub(crate) precondition_blocked: bool,
    pub(crate) observed_kind: Option<String>,
    pub(crate) diagnostic: Option<String>,
    pub(crate) help_probed: bool,
    pub(crate) help_behavior: Option<String>,
    pub(crate) help_parse_success: bool,
    pub(crate) help_diagnostic: Option<String>,
    pub(crate) evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CommandGap {
    pub(crate) kind: String,
    pub(crate) reason: String,
    pub(crate) evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OutcomeNote {
    pub(crate) level: &'static str,
    pub(crate) text: String,
}
