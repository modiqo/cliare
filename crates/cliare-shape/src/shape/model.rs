use serde::Serialize;

use crate::claims::OutputContractScope;
use cliare_inference::output::{ObservedOutputKind, OutputHelpBehavior, OutputMode};
use cliare_inference::precondition::PreconditionKind;
use cliare_runtime::fingerprint::TargetFingerprint;

#[derive(Debug, Serialize)]
pub struct CommandShape {
    pub(super) schema_version: &'static str,
    pub(super) target: TargetFingerprint,
    pub(super) commands: Vec<CommandCandidate>,
    pub(super) flags: Vec<FlagCandidate>,
    pub(super) output_contracts: Vec<OutputContractCandidate>,
    pub(super) gaps: Vec<Gap>,
    pub(super) model: InferenceModel,
}

#[derive(Debug, Serialize)]
pub struct CommandCandidate {
    pub(super) id: String,
    pub(super) path: Vec<String>,
    pub(super) argv: Vec<String>,
    pub(super) summary: Option<String>,
    pub(super) aliases: Vec<String>,
    pub(super) positionals: Vec<PositionalArgument>,
    pub(super) usage_observed: bool,
    pub(super) confidence: f64,
    pub(super) runtime_confirmed: bool,
    pub(super) runtime_state: CommandRuntimeState,
    pub(super) preconditions: Vec<PreconditionKind>,
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FlagCandidate {
    pub(super) command_path: Vec<String>,
    pub(super) name: String,
    pub(super) short: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) value_kind: FlagValueKindShape,
    pub(super) value_name: Option<String>,
    pub(super) required: bool,
    pub(super) repeatable: bool,
    pub(super) confidence: f64,
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionalArgument {
    pub(super) name: String,
    pub(super) required: bool,
    pub(super) variadic: bool,
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OutputContractCandidate {
    pub(super) command_path: Vec<String>,
    pub(super) mode: OutputMode,
    pub(super) flag_name: String,
    pub(super) argv_fragment: Vec<String>,
    pub(super) scope: OutputContractScope,
    pub(super) advertised: bool,
    pub(super) probed: bool,
    pub(super) parse_success: bool,
    pub(super) precondition_blocked: bool,
    pub(super) preconditions: Vec<PreconditionKind>,
    pub(super) observed_kind: Option<ObservedOutputKind>,
    pub(super) diagnostic: Option<String>,
    pub(super) help_probed: bool,
    pub(super) help_behavior: Option<OutputHelpBehavior>,
    pub(super) help_parse_success: bool,
    pub(super) help_diagnostic: Option<String>,
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandIndex {
    pub(super) schema_version: &'static str,
    pub(super) source_schema_version: &'static str,
    pub(super) target: TargetFingerprint,
    pub(super) summary: CommandIndexSummary,
    pub(super) commands: Vec<CommandIndexEntry>,
    pub(super) model: InferenceModel,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexSummary {
    pub(super) commands_total: usize,
    pub(super) ready: usize,
    pub(super) conditional: usize,
    pub(super) needs_fixture: usize,
    pub(super) blocked: usize,
    pub(super) candidate: usize,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexEntry {
    pub(super) id: String,
    pub(super) command: String,
    pub(super) path: Vec<String>,
    pub(super) argv: Vec<String>,
    pub(super) summary: Option<String>,
    pub(super) runtime_state: CommandRuntimeState,
    pub(super) agent_suitability: AgentSuitability,
    pub(super) suitability_reasons: Vec<String>,
    pub(super) confidence: f64,
    pub(super) usage_observed: bool,
    pub(super) parameters: CommandParameters,
    pub(super) preconditions: Vec<PreconditionKind>,
    pub(super) output_contracts: Vec<CommandIndexOutputContract>,
    pub(super) gaps: Vec<CommandIndexGap>,
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandParameters {
    pub(super) positionals: Vec<PositionalArgument>,
    pub(super) flags: Vec<CommandIndexFlag>,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexFlag {
    pub(super) name: String,
    pub(super) short: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) value_kind: FlagValueKindShape,
    pub(super) value_name: Option<String>,
    pub(super) required: bool,
    pub(super) repeatable: bool,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexOutputContract {
    pub(super) mode: OutputMode,
    pub(super) flag_name: String,
    pub(super) argv_fragment: Vec<String>,
    pub(super) scope: OutputContractScope,
    pub(super) status: OutputContractStatus,
    pub(super) preconditions: Vec<PreconditionKind>,
    pub(super) observed_kind: Option<ObservedOutputKind>,
    pub(super) diagnostic: Option<String>,
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSuitability {
    Ready,
    Conditional,
    NeedsFixture,
    Blocked,
    Candidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputContractStatus {
    ParseSuccess,
    PreconditionBlocked,
    Unprobed,
    HelpText,
    Unvalidated,
    ParseFailed,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexGap {
    pub(super) kind: GapKind,
    pub(super) reason: String,
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagValueKindShape {
    Boolean,
    Required,
    Optional,
}

#[derive(Debug, Serialize)]
pub struct Gap {
    pub(super) kind: GapKind,
    pub(super) command_path: Vec<String>,
    pub(super) reason: String,
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    ExistenceUnconfirmed,
    HelpUnavailable,
    AlternateHelpFormUnavailable,
    PreconditionBlocked,
    FlagsUnknown,
    ArgumentArityUnknown,
    InvalidChildDiagnosticsUnknown,
    InvalidFlagDiagnosticsUnknown,
    OutputModeUnprobed,
    OutputModeUnvalidated,
    OutputModeParseFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRuntimeState {
    RuntimeConfirmed,
    PreconditionBlocked,
    Unconfirmed,
}

#[derive(Debug, Serialize)]
pub struct InferenceModel {
    pub(super) name: &'static str,
    pub(super) source: &'static str,
}
