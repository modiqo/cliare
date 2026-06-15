use std::path::Path;

use serde::Serialize;
use tokio::fs;

use crate::artifacts::{COMMAND_INDEX_JSON, COMMAND_INDEX_MD, SHAPE_JSON};
use crate::claims::{
    ClaimSet, CommandClaim, FlagClaim, FlagValueKind, OutputContractClaim, OutputContractScope,
    PositionalClaim,
};
use crate::error::{CliareError, Result};
use crate::fingerprint::TargetFingerprint;
use crate::observation::ShapeObservation;
use crate::output::{ObservedOutputKind, OutputHelpBehavior, OutputMode};
use crate::precondition::PreconditionKind;

const SCHEMA_VERSION: &str = "cliare.command-shape.v1";
const COMMAND_INDEX_SCHEMA_VERSION: &str = "cliare.command-index.v1";
const INFERENCE_MODEL: &str = "cliare-generic-claims-v0";

#[derive(Debug, Serialize)]
pub struct CommandShape {
    schema_version: &'static str,
    target: TargetFingerprint,
    commands: Vec<CommandCandidate>,
    flags: Vec<FlagCandidate>,
    output_contracts: Vec<OutputContractCandidate>,
    gaps: Vec<Gap>,
    model: InferenceModel,
}

#[derive(Debug, Serialize)]
pub struct CommandCandidate {
    id: String,
    path: Vec<String>,
    argv: Vec<String>,
    summary: Option<String>,
    aliases: Vec<String>,
    positionals: Vec<PositionalArgument>,
    usage_observed: bool,
    confidence: f64,
    runtime_confirmed: bool,
    runtime_state: CommandRuntimeState,
    preconditions: Vec<PreconditionKind>,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FlagCandidate {
    command_path: Vec<String>,
    name: String,
    short: Option<String>,
    summary: Option<String>,
    value_kind: FlagValueKindShape,
    value_name: Option<String>,
    required: bool,
    repeatable: bool,
    confidence: f64,
    evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionalArgument {
    name: String,
    required: bool,
    variadic: bool,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OutputContractCandidate {
    command_path: Vec<String>,
    mode: OutputMode,
    flag_name: String,
    argv_fragment: Vec<String>,
    scope: OutputContractScope,
    advertised: bool,
    probed: bool,
    parse_success: bool,
    precondition_blocked: bool,
    preconditions: Vec<PreconditionKind>,
    observed_kind: Option<ObservedOutputKind>,
    diagnostic: Option<String>,
    help_probed: bool,
    help_behavior: Option<OutputHelpBehavior>,
    help_parse_success: bool,
    help_diagnostic: Option<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandIndex {
    schema_version: &'static str,
    source_schema_version: &'static str,
    target: TargetFingerprint,
    summary: CommandIndexSummary,
    commands: Vec<CommandIndexEntry>,
    model: InferenceModel,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexSummary {
    commands_total: usize,
    ready: usize,
    conditional: usize,
    needs_fixture: usize,
    blocked: usize,
    candidate: usize,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexEntry {
    id: String,
    command: String,
    path: Vec<String>,
    argv: Vec<String>,
    summary: Option<String>,
    runtime_state: CommandRuntimeState,
    agent_suitability: AgentSuitability,
    suitability_reasons: Vec<String>,
    confidence: f64,
    usage_observed: bool,
    parameters: CommandParameters,
    preconditions: Vec<PreconditionKind>,
    output_contracts: Vec<CommandIndexOutputContract>,
    gaps: Vec<CommandIndexGap>,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandParameters {
    positionals: Vec<PositionalArgument>,
    flags: Vec<CommandIndexFlag>,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexFlag {
    name: String,
    short: Option<String>,
    summary: Option<String>,
    value_kind: FlagValueKindShape,
    value_name: Option<String>,
    required: bool,
    repeatable: bool,
}

#[derive(Debug, Serialize)]
pub struct CommandIndexOutputContract {
    mode: OutputMode,
    flag_name: String,
    argv_fragment: Vec<String>,
    scope: OutputContractScope,
    status: OutputContractStatus,
    preconditions: Vec<PreconditionKind>,
    observed_kind: Option<ObservedOutputKind>,
    diagnostic: Option<String>,
    evidence: Vec<String>,
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
    kind: GapKind,
    reason: String,
    evidence: Vec<String>,
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
    kind: GapKind,
    command_path: Vec<String>,
    reason: String,
    evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    ExistenceUnconfirmed,
    HelpUnavailable,
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
    name: &'static str,
    source: &'static str,
}

pub async fn write_shape(
    out_dir: &Path,
    target: TargetFingerprint,
    observations: &[ShapeObservation],
) -> Result<()> {
    let shape = infer_shape(target, observations);
    let index = command_index(&shape);

    let shape_path = out_dir.join(SHAPE_JSON);
    let shape_bytes = serde_json::to_vec_pretty(&shape).map_err(CliareError::SerializeShape)?;
    fs::write(&shape_path, shape_bytes)
        .await
        .map_err(|source| CliareError::WriteShape {
            path: shape_path,
            source,
        })?;

    let index_path = out_dir.join(COMMAND_INDEX_JSON);
    let index_bytes = serde_json::to_vec_pretty(&index).map_err(CliareError::SerializeShape)?;
    fs::write(&index_path, index_bytes)
        .await
        .map_err(|source| CliareError::WriteShape {
            path: index_path,
            source,
        })?;

    let index_markdown_path = out_dir.join(COMMAND_INDEX_MD);
    fs::write(&index_markdown_path, render_command_index_markdown(&index))
        .await
        .map_err(|source| CliareError::WriteShape {
            path: index_markdown_path,
            source,
        })
}

pub fn infer_shape(target: TargetFingerprint, observations: &[ShapeObservation]) -> CommandShape {
    let binary_name = target_binary_name(&target);
    let claims = ClaimSet::from_observations(&binary_name, observations);
    let commands = claims
        .commands()
        .map(|command| command_candidate(&binary_name, command))
        .collect::<Vec<_>>();
    let flags = claims.flags().map(flag_candidate).collect::<Vec<_>>();
    let output_contracts = claims
        .output_contracts()
        .map(output_contract_candidate)
        .collect::<Vec<_>>();
    let gaps = gap_items(claims.commands(), &flags, &output_contracts);

    CommandShape {
        schema_version: SCHEMA_VERSION,
        target,
        commands,
        flags,
        output_contracts,
        gaps,
        model: InferenceModel {
            name: INFERENCE_MODEL,
            source: "generic claim store with layout evidence, runtime confirmation, and diagnostic probes",
        },
    }
}

pub fn infer_command_index(
    target: TargetFingerprint,
    observations: &[ShapeObservation],
) -> CommandIndex {
    let shape = infer_shape(target, observations);
    command_index(&shape)
}

fn command_index(shape: &CommandShape) -> CommandIndex {
    let commands = shape
        .commands
        .iter()
        .map(|command| command_index_entry(command, shape))
        .collect::<Vec<_>>();
    let summary = command_index_summary(&commands);

    CommandIndex {
        schema_version: COMMAND_INDEX_SCHEMA_VERSION,
        source_schema_version: SCHEMA_VERSION,
        target: shape.target.clone(),
        summary,
        commands,
        model: InferenceModel {
            name: INFERENCE_MODEL,
            source: "command-centric index derived from command shape, runtime gaps, output contracts, and preconditions",
        },
    }
}

fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}

fn command_candidate(binary_name: &str, command: &CommandClaim) -> CommandCandidate {
    let path = command.path().to_vec();
    let mut argv = Vec::with_capacity(path.len() + 1);
    argv.push(binary_name.to_owned());
    argv.extend(path.iter().cloned());

    CommandCandidate {
        id: command_id(binary_name, &path),
        path,
        argv,
        summary: command.summary().map(str::to_owned),
        aliases: command.aliases().cloned().collect(),
        positionals: command.positionals().map(positional_argument).collect(),
        usage_observed: command.usage_observed(),
        confidence: command.confidence(),
        runtime_confirmed: command.runtime_confirmed(),
        runtime_state: command_runtime_state(command),
        preconditions: command.preconditions().collect(),
        evidence: command.evidence().to_vec(),
    }
}

fn command_runtime_state(command: &CommandClaim) -> CommandRuntimeState {
    if command.runtime_confirmed() {
        CommandRuntimeState::RuntimeConfirmed
    } else if command.precondition_blocked() {
        CommandRuntimeState::PreconditionBlocked
    } else {
        CommandRuntimeState::Unconfirmed
    }
}

fn flag_candidate(flag: &FlagClaim) -> FlagCandidate {
    FlagCandidate {
        command_path: flag.command_path().to_vec(),
        name: flag.name().to_owned(),
        short: flag.short().map(str::to_owned),
        summary: flag.summary().map(str::to_owned),
        value_kind: flag_value_kind(flag.value_kind()),
        value_name: flag.value_name().map(str::to_owned),
        required: flag.required(),
        repeatable: flag.repeatable(),
        confidence: flag.confidence(),
        evidence: flag.evidence().to_vec(),
    }
}

fn positional_argument(argument: &PositionalClaim) -> PositionalArgument {
    PositionalArgument {
        name: argument.name().to_owned(),
        required: argument.required(),
        variadic: argument.variadic(),
        evidence: argument.evidence().to_vec(),
    }
}

fn output_contract_candidate(contract: &OutputContractClaim) -> OutputContractCandidate {
    OutputContractCandidate {
        command_path: contract.command_path().to_vec(),
        mode: contract.mode(),
        flag_name: contract.flag_name().to_owned(),
        argv_fragment: contract.argv_fragment().to_vec(),
        scope: contract.scope(),
        advertised: contract.advertised(),
        probed: contract.probed(),
        parse_success: contract.parse_success(),
        precondition_blocked: contract.precondition_blocked(),
        preconditions: contract.preconditions().collect(),
        observed_kind: contract.observed_kind(),
        diagnostic: contract.diagnostic().map(str::to_owned),
        help_probed: contract.help_probed(),
        help_behavior: contract.help_behavior(),
        help_parse_success: contract.help_parse_success(),
        help_diagnostic: contract.help_diagnostic().map(str::to_owned),
        evidence: contract.evidence().to_vec(),
    }
}

fn command_index_entry(command: &CommandCandidate, shape: &CommandShape) -> CommandIndexEntry {
    let flags = shape
        .flags
        .iter()
        .filter(|flag| flag.command_path == command.path)
        .map(command_index_flag)
        .collect::<Vec<_>>();
    let output_contracts = shape
        .output_contracts
        .iter()
        .filter(|contract| contract.command_path == command.path)
        .map(command_index_output_contract)
        .collect::<Vec<_>>();
    let gaps = shape
        .gaps
        .iter()
        .filter(|gap| gap.command_path == command.path)
        .map(command_index_gap)
        .collect::<Vec<_>>();
    let preconditions = command_preconditions(command, &output_contracts, &gaps);
    let (agent_suitability, suitability_reasons) =
        command_suitability(command, &output_contracts, &gaps, &preconditions);

    CommandIndexEntry {
        id: command.id.clone(),
        command: command_display(&command.path),
        path: command.path.clone(),
        argv: command.argv.clone(),
        summary: command.summary.clone(),
        runtime_state: command.runtime_state,
        agent_suitability,
        suitability_reasons,
        confidence: command.confidence,
        usage_observed: command.usage_observed,
        parameters: CommandParameters {
            positionals: command.positionals.clone(),
            flags,
        },
        preconditions,
        output_contracts,
        gaps,
        evidence: command.evidence.clone(),
    }
}

fn command_index_summary(commands: &[CommandIndexEntry]) -> CommandIndexSummary {
    CommandIndexSummary {
        commands_total: commands.len(),
        ready: commands
            .iter()
            .filter(|command| command.agent_suitability == AgentSuitability::Ready)
            .count(),
        conditional: commands
            .iter()
            .filter(|command| command.agent_suitability == AgentSuitability::Conditional)
            .count(),
        needs_fixture: commands
            .iter()
            .filter(|command| command.agent_suitability == AgentSuitability::NeedsFixture)
            .count(),
        blocked: commands
            .iter()
            .filter(|command| command.agent_suitability == AgentSuitability::Blocked)
            .count(),
        candidate: commands
            .iter()
            .filter(|command| command.agent_suitability == AgentSuitability::Candidate)
            .count(),
    }
}

fn command_index_flag(flag: &FlagCandidate) -> CommandIndexFlag {
    CommandIndexFlag {
        name: flag.name.clone(),
        short: flag.short.clone(),
        summary: flag.summary.clone(),
        value_kind: flag.value_kind,
        value_name: flag.value_name.clone(),
        required: flag.required,
        repeatable: flag.repeatable,
    }
}

fn command_index_output_contract(contract: &OutputContractCandidate) -> CommandIndexOutputContract {
    CommandIndexOutputContract {
        mode: contract.mode,
        flag_name: contract.flag_name.clone(),
        argv_fragment: contract.argv_fragment.clone(),
        scope: contract.scope,
        status: output_contract_status(contract),
        preconditions: contract.preconditions.clone(),
        observed_kind: contract.observed_kind,
        diagnostic: contract.diagnostic.clone(),
        evidence: contract.evidence.clone(),
    }
}

fn output_contract_status(contract: &OutputContractCandidate) -> OutputContractStatus {
    if contract.parse_success {
        OutputContractStatus::ParseSuccess
    } else if contract.precondition_blocked {
        OutputContractStatus::PreconditionBlocked
    } else if !contract.probed {
        OutputContractStatus::Unprobed
    } else if contract.observed_kind == Some(ObservedOutputKind::HelpText) {
        OutputContractStatus::HelpText
    } else if contract.scope.is_global_only() {
        OutputContractStatus::Unvalidated
    } else {
        OutputContractStatus::ParseFailed
    }
}

fn command_index_gap(gap: &Gap) -> CommandIndexGap {
    CommandIndexGap {
        kind: gap.kind,
        reason: gap.reason.clone(),
        evidence: gap.evidence.clone(),
    }
}

fn command_preconditions(
    command: &CommandCandidate,
    output_contracts: &[CommandIndexOutputContract],
    _gaps: &[CommandIndexGap],
) -> Vec<PreconditionKind> {
    let mut preconditions = command.preconditions.clone();
    for contract in output_contracts {
        for precondition in &contract.preconditions {
            if !preconditions.contains(precondition) {
                preconditions.push(*precondition);
            }
        }
    }
    preconditions
}

fn command_suitability(
    command: &CommandCandidate,
    output_contracts: &[CommandIndexOutputContract],
    gaps: &[CommandIndexGap],
    preconditions: &[PreconditionKind],
) -> (AgentSuitability, Vec<String>) {
    let mut reasons = Vec::new();

    if command.runtime_state == CommandRuntimeState::Unconfirmed {
        reasons.push("command existence is inferred but not runtime-confirmed".to_owned());
        return (AgentSuitability::Candidate, reasons);
    }

    let has_precondition_gap = command.runtime_state == CommandRuntimeState::PreconditionBlocked
        || !preconditions.is_empty()
        || gaps
            .iter()
            .any(|gap| matches!(gap.kind, GapKind::PreconditionBlocked));
    if has_precondition_gap && preconditions_are_agent_satisfiable(preconditions) {
        reasons.push(format_preconditions(preconditions));
        return (AgentSuitability::Conditional, reasons);
    }

    if has_precondition_gap {
        reasons.push(format_preconditions(preconditions));
        return (AgentSuitability::Blocked, reasons);
    }

    if gaps.iter().any(|gap| {
        matches!(
            gap.kind,
            GapKind::OutputModeUnprobed | GapKind::OutputModeUnvalidated
        )
    }) {
        reasons.push(
            "machine-readable output contract needs fixture or command-local validation".to_owned(),
        );
        return (AgentSuitability::NeedsFixture, reasons);
    }

    let blocking_gap = gaps.iter().find(|gap| {
        matches!(
            gap.kind,
            GapKind::HelpUnavailable
                | GapKind::FlagsUnknown
                | GapKind::ArgumentArityUnknown
                | GapKind::InvalidChildDiagnosticsUnknown
                | GapKind::InvalidFlagDiagnosticsUnknown
                | GapKind::OutputModeParseFailed
        )
    });
    if let Some(gap) = blocking_gap {
        reasons.push(format!("{}: {}", gap_kind_label(gap.kind), gap.reason));
        return (AgentSuitability::Conditional, reasons);
    }

    if output_contracts
        .iter()
        .any(|contract| contract.status == OutputContractStatus::ParseSuccess)
    {
        reasons.push("runtime-confirmed with parseable machine-readable output".to_owned());
    } else {
        reasons.push("runtime-confirmed command shape".to_owned());
    }
    (AgentSuitability::Ready, reasons)
}

fn preconditions_are_agent_satisfiable(preconditions: &[PreconditionKind]) -> bool {
    !preconditions.is_empty()
        && preconditions.iter().all(|precondition| {
            matches!(
                precondition,
                PreconditionKind::AuthRequired | PreconditionKind::LocalContextRequired
            )
        })
}

fn format_preconditions(preconditions: &[PreconditionKind]) -> String {
    if preconditions.is_empty() {
        "runtime precondition observed; inspect evidence for the exact blocker".to_owned()
    } else {
        format!(
            "requires runtime precondition: {}",
            preconditions
                .iter()
                .map(|precondition| precondition.label())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn render_command_index_markdown(index: &CommandIndex) -> String {
    let mut lines = vec![
        "# CLIARE Command Index".to_owned(),
        String::new(),
        "Command-centric reference generated from `shape.json`. Use it as the first-pass index for selecting commands, understanding parameters, and identifying runtime constraints before invoking a CLI from an agent harness.".to_owned(),
        String::new(),
        format!("- Target: `{}`", index.target.requested.display()),
        format!("- Resolved: `{}`", index.target.resolved.display()),
        format!("- Commands: `{}`", index.summary.commands_total),
        format!("- Ready: `{}`", index.summary.ready),
        format!("- Conditional: `{}`", index.summary.conditional),
        format!("- Needs fixture: `{}`", index.summary.needs_fixture),
        format!("- Blocked: `{}`", index.summary.blocked),
        format!("- Candidate only: `{}`", index.summary.candidate),
        String::new(),
        "## Suitability Legend".to_owned(),
        String::new(),
        "| Suitability | Meaning | Harness treatment |".to_owned(),
        "|---|---|---|".to_owned(),
        "| `ready` | Runtime-confirmed command with no blocking gaps. | Candidate for automatic routing, subject to harness permissions and task policy. |".to_owned(),
        "| `conditional` | Runtime-confirmed or runtime-recognized command with a satisfiable condition such as auth, local context, unavailable help, unknown grammar, missing diagnostics, or output parse failure. | Expose when the harness can satisfy the condition or policy allows manual review; prefer safer alternatives for unattended actions. |".to_owned(),
        "| `needs_fixture` | Advertised contract or behavior needs safe operands, fixture data, or command-local validation before CLIARE can verify it. | Add fixtures or documented safe operands before routing automatically. |".to_owned(),
        "| `blocked` | Opaque, infrastructure-level, or currently unsatisfied runtime condition blocked safe confirmation or use. | Treat as unavailable until the runtime condition is provisioned or the diagnostic becomes explicit enough to classify. |".to_owned(),
        "| `candidate` | Inferred from help/layout evidence but not runtime-confirmed. | Keep out of automatic routing until deeper measurement or catalog metadata confirms it. |".to_owned(),
        String::new(),
        "## Commands".to_owned(),
        String::new(),
        "| Command | Suitability | Runtime | Parameters | Preconditions | Output | Gaps | Summary |".to_owned(),
        "|---|---|---|---|---|---|---|---|".to_owned(),
    ];

    for command in &index.commands {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | {} | {} | {} | {} | {} |",
            escape_markdown_table(&command.command),
            suitability_label(command.agent_suitability),
            runtime_state_label(command.runtime_state),
            escape_markdown_table(&parameters_summary(&command.parameters)),
            escape_markdown_table(&preconditions_summary(&command.preconditions)),
            escape_markdown_table(&output_summary(&command.output_contracts)),
            escape_markdown_table(&gaps_summary(&command.gaps)),
            escape_markdown_table(command.summary.as_deref().unwrap_or(""))
        ));
    }

    lines.push(String::new());
    lines.join("\n")
}

fn parameters_summary(parameters: &CommandParameters) -> String {
    let positionals = parameters
        .positionals
        .iter()
        .map(format_positional)
        .collect::<Vec<_>>();
    let flags = parameters.flags.iter().map(format_flag).collect::<Vec<_>>();

    match (positionals.is_empty(), flags.is_empty()) {
        (true, true) => "none".to_owned(),
        (false, true) => format!("args: {}", positionals.join(", ")),
        (true, false) => format!("flags: {}", flags.join(", ")),
        (false, false) => format!(
            "args: {}; flags: {}",
            positionals.join(", "),
            flags.join(", ")
        ),
    }
}

fn format_positional(positional: &PositionalArgument) -> String {
    let mut name = positional.name.clone();
    if positional.variadic {
        name.push_str("...");
    }
    if positional.required {
        format!("<{name}>")
    } else {
        format!("[{name}]")
    }
}

fn format_flag(flag: &CommandIndexFlag) -> String {
    let mut rendered = flag.name.clone();
    if let Some(value_name) = &flag.value_name {
        match flag.value_kind {
            FlagValueKindShape::Boolean => {}
            FlagValueKindShape::Required => {
                rendered.push(' ');
                rendered.push_str(value_name);
            }
            FlagValueKindShape::Optional => {
                rendered.push_str(" [");
                rendered.push_str(value_name);
                rendered.push(']');
            }
        }
    }
    if flag.repeatable {
        rendered.push_str("...");
    }
    rendered
}

fn preconditions_summary(preconditions: &[PreconditionKind]) -> String {
    if preconditions.is_empty() {
        "none".to_owned()
    } else {
        preconditions
            .iter()
            .map(|precondition| precondition.label())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn output_summary(contracts: &[CommandIndexOutputContract]) -> String {
    if contracts.is_empty() {
        return "none".to_owned();
    }
    contracts
        .iter()
        .map(|contract| {
            format!(
                "{}:{}",
                contract.mode.label(),
                output_status_label(contract.status)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn gaps_summary(gaps: &[CommandIndexGap]) -> String {
    if gaps.is_empty() {
        return "none".to_owned();
    }
    gaps.iter()
        .map(|gap| gap_kind_label(gap.kind))
        .collect::<Vec<_>>()
        .join(", ")
}

fn command_display(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(" ")
    }
}

fn suitability_label(suitability: AgentSuitability) -> &'static str {
    match suitability {
        AgentSuitability::Ready => "ready",
        AgentSuitability::Conditional => "conditional",
        AgentSuitability::NeedsFixture => "needs_fixture",
        AgentSuitability::Blocked => "blocked",
        AgentSuitability::Candidate => "candidate",
    }
}

fn runtime_state_label(state: CommandRuntimeState) -> &'static str {
    match state {
        CommandRuntimeState::RuntimeConfirmed => "runtime_confirmed",
        CommandRuntimeState::PreconditionBlocked => "precondition_blocked",
        CommandRuntimeState::Unconfirmed => "unconfirmed",
    }
}

fn output_status_label(status: OutputContractStatus) -> &'static str {
    match status {
        OutputContractStatus::ParseSuccess => "parse_success",
        OutputContractStatus::PreconditionBlocked => "precondition_blocked",
        OutputContractStatus::Unprobed => "unprobed",
        OutputContractStatus::HelpText => "help_text",
        OutputContractStatus::Unvalidated => "unvalidated",
        OutputContractStatus::ParseFailed => "parse_failed",
    }
}

fn gap_kind_label(kind: GapKind) -> &'static str {
    match kind {
        GapKind::ExistenceUnconfirmed => "existence_unconfirmed",
        GapKind::HelpUnavailable => "help_unavailable",
        GapKind::PreconditionBlocked => "precondition_blocked",
        GapKind::FlagsUnknown => "flags_unknown",
        GapKind::ArgumentArityUnknown => "argument_arity_unknown",
        GapKind::InvalidChildDiagnosticsUnknown => "invalid_child_diagnostics_unknown",
        GapKind::InvalidFlagDiagnosticsUnknown => "invalid_flag_diagnostics_unknown",
        GapKind::OutputModeUnprobed => "output_mode_unprobed",
        GapKind::OutputModeUnvalidated => "output_mode_unvalidated",
        GapKind::OutputModeParseFailed => "output_mode_parse_failed",
    }
}

fn escape_markdown_table(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', " ")
}

fn flag_value_kind(kind: FlagValueKind) -> FlagValueKindShape {
    match kind {
        FlagValueKind::Boolean => FlagValueKindShape::Boolean,
        FlagValueKind::Required => FlagValueKindShape::Required,
        FlagValueKind::Optional => FlagValueKindShape::Optional,
    }
}

fn gap_items<'a>(
    commands: impl Iterator<Item = &'a CommandClaim>,
    flags: &[FlagCandidate],
    output_contracts: &[OutputContractCandidate],
) -> Vec<Gap> {
    let mut gaps = Vec::new();

    for command in commands {
        if command.confidence() < 0.80 {
            gaps.push(Gap {
                kind: GapKind::ExistenceUnconfirmed,
                command_path: command.path().to_vec(),
                reason: "candidate has not accumulated enough confirming runtime evidence"
                    .to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.precondition_blocked() {
            gaps.push(Gap {
                kind: GapKind::PreconditionBlocked,
                command_path: command.path().to_vec(),
                reason: "safe probe was blocked by a runtime precondition".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        } else if command.help_unavailable() {
            gaps.push(Gap {
                kind: GapKind::HelpUnavailable,
                command_path: command.path().to_vec(),
                reason: "safe help probe did not produce help-like output".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.runtime_confirmed() && has_unknown_flag_grammar(flags, command.path().as_slice())
        {
            gaps.push(Gap {
                kind: GapKind::FlagsUnknown,
                command_path: command.path().to_vec(),
                reason: "some discovered flags still lack value grammar".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.runtime_confirmed() && !command.usage_observed() {
            gaps.push(Gap {
                kind: GapKind::ArgumentArityUnknown,
                command_path: command.path().to_vec(),
                reason: "usage syntax has not confirmed positional arguments".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.runtime_confirmed()
            && command.has_child_candidates()
            && !command.invalid_child_rejected()
        {
            gaps.push(Gap {
                kind: GapKind::InvalidChildDiagnosticsUnknown,
                command_path: command.path().to_vec(),
                reason: "safe invalid-child probe has not observed command diagnostics".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.runtime_confirmed() && !command.invalid_flag_rejected() {
            gaps.push(Gap {
                kind: GapKind::InvalidFlagDiagnosticsUnknown,
                command_path: command.path().to_vec(),
                reason: "safe invalid-flag probe has not observed flag diagnostics".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
    }

    for contract in output_contracts {
        if !contract.probed {
            gaps.push(Gap {
                kind: GapKind::OutputModeUnprobed,
                command_path: contract.command_path.clone(),
                reason: "advertised output mode has not been runtime-probed".to_owned(),
                evidence: contract.evidence.clone(),
            });
        } else if contract.precondition_blocked {
            gaps.push(Gap {
                kind: GapKind::PreconditionBlocked,
                command_path: contract.command_path.clone(),
                reason: "advertised output mode was blocked by a runtime precondition".to_owned(),
                evidence: contract.evidence.clone(),
            });
        } else if !contract.parse_success
            && matches!(contract.observed_kind, Some(ObservedOutputKind::HelpText))
        {
            gaps.push(Gap {
                kind: GapKind::OutputModeUnvalidated,
                command_path: contract.command_path.clone(),
                reason: "advertised output mode resolved to help text under a safe probe"
                    .to_owned(),
                evidence: contract.evidence.clone(),
            });
        } else if !contract.parse_success && contract.scope.is_global_only() {
            gaps.push(Gap {
                kind: GapKind::OutputModeUnvalidated,
                command_path: contract.command_path.clone(),
                reason: "global output flag did not establish command-specific machine output"
                    .to_owned(),
                evidence: contract.evidence.clone(),
            });
        } else if !contract.parse_success {
            gaps.push(Gap {
                kind: GapKind::OutputModeParseFailed,
                command_path: contract.command_path.clone(),
                reason:
                    "advertised output mode did not produce parseable output during a safe probe"
                        .to_owned(),
                evidence: contract.evidence.clone(),
            });
        }
    }

    gaps
}

fn has_unknown_flag_grammar(flags: &[FlagCandidate], command_path: &[String]) -> bool {
    flags
        .iter()
        .filter(|flag| flag.command_path == command_path)
        .any(|flag| {
            !matches!(flag.value_kind, FlagValueKindShape::Boolean) && flag.value_name.is_none()
        })
}

fn command_id(binary_name: &str, path: &[String]) -> String {
    let mut id = binary_name.to_owned();
    for segment in path {
        id.push('.');
        id.push_str(segment);
    }
    id
}

#[cfg(test)]
mod tests {
    use super::infer_shape;
    use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
    use crate::fingerprint::TargetFingerprint;
    use crate::observation::ShapeObservation;
    use crate::process::OutputCapture;

    #[test]
    fn generic_layout_candidates_are_low_confidence_until_confirmed() {
        let target = target();
        let root = observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  measure  Run probes\n\nOptions:\n  -h, --help     Print help\n",
            Some(0),
        );

        let shape = infer_shape(target, &[root]);

        let measure = shape
            .commands
            .iter()
            .find(|command| command.path == ["measure"])
            .expect("measure candidate exists");
        assert!(!measure.runtime_confirmed);
        assert!(measure.confidence < 0.80);
        assert!(shape.flags.iter().any(|flag| flag.name == "--help"));
        assert!(shape.gaps.iter().any(|gap| gap.command_path == ["measure"]));
    }

    #[test]
    fn runtime_help_confirmation_raises_command_confidence() {
        let target = target();
        let root = observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  measure  Run probes\n",
            Some(0),
        );
        let measure_help = observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
            Some(0),
        );

        let shape = infer_shape(target, &[root, measure_help]);

        let measure = shape
            .commands
            .iter()
            .find(|command| command.path == ["measure"])
            .expect("measure candidate exists");
        assert!(measure.runtime_confirmed);
        assert!(measure.confidence > 0.90);
    }

    #[test]
    fn auth_blocked_help_is_shape_precondition_not_help_unavailable() {
        let target = target();
        let root = observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  model  Track AI model identity\n",
            Some(0),
        );
        let model_help = observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["model".to_owned()],
            "error: rote requires login\n\nrun rote login",
            Some(77),
        );

        let shape = infer_shape(target, &[root, model_help]);
        let model = shape
            .commands
            .iter()
            .find(|command| command.path == ["model"])
            .expect("model candidate exists");

        assert!(!model.runtime_confirmed);
        assert!(matches!(
            model.runtime_state,
            super::CommandRuntimeState::PreconditionBlocked
        ));
        assert_eq!(model.preconditions.len(), 1);
        assert!(shape.gaps.iter().any(|gap| {
            gap.command_path == ["model"] && matches!(gap.kind, super::GapKind::PreconditionBlocked)
        }));
        assert!(!shape.gaps.iter().any(|gap| {
            gap.command_path == ["model"] && matches!(gap.kind, super::GapKind::HelpUnavailable)
        }));
    }

    #[test]
    fn local_context_precondition_is_conditional_in_command_index() {
        let target = target();
        let root = observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  stats  Show workspace statistics\n",
            Some(0),
        );
        let stats = observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["stats".to_owned()],
            "error: not in a workspace directory\n\nFix:\n  cliare init demo\n  cd workspaces/demo\n\nhint: or list existing: 'cliare ls'\n",
            Some(1),
        );

        let index = super::infer_command_index(target, &[root, stats]);
        let stats = index
            .commands
            .iter()
            .find(|command| command.path == ["stats"])
            .expect("stats command exists");

        assert!(matches!(
            stats.runtime_state,
            super::CommandRuntimeState::PreconditionBlocked
        ));
        assert_eq!(
            stats.agent_suitability,
            super::AgentSuitability::Conditional
        );
        assert_eq!(
            stats.preconditions,
            vec![crate::precondition::PreconditionKind::LocalContextRequired]
        );
    }

    #[test]
    fn shape_includes_usage_positionals_and_flag_grammar() {
        let target = target();
        let deploy_help = observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["project".to_owned(), "deploy".to_owned()],
            "Usage: cliare project deploy <PROJECT> [ENV] [FILES]...\n\nOptions:\n  -f, --format <KIND>       Output format\n  --color[=<WHEN>]          Optional color mode\n  --tag <TAG>...            Repeatable tag\n  --token <TOKEN>           Required authentication token\n  --dry-run                 Do not write changes\n",
            Some(0),
        );

        let shape = infer_shape(target, &[deploy_help]);
        let deploy = shape
            .commands
            .iter()
            .find(|command| command.path == ["project", "deploy"])
            .expect("deploy command exists");

        assert!(deploy.usage_observed);
        assert!(deploy.positionals.iter().any(|argument| {
            argument.name == "project" && argument.required && !argument.variadic
        }));
        assert!(
            deploy
                .positionals
                .iter()
                .any(|argument| argument.name == "env" && !argument.required)
        );
        assert!(
            deploy
                .positionals
                .iter()
                .any(|argument| argument.name == "files" && argument.variadic)
        );

        let format = shape
            .flags
            .iter()
            .find(|flag| flag.name == "--format")
            .expect("format flag exists");
        assert!(matches!(
            format.value_kind,
            super::FlagValueKindShape::Required
        ));
        assert_eq!(format.value_name.as_deref(), Some("kind"));
        assert_eq!(format.short.as_deref(), Some("-f"));

        let color = shape
            .flags
            .iter()
            .find(|flag| flag.name == "--color")
            .expect("color flag exists");
        assert!(matches!(
            color.value_kind,
            super::FlagValueKindShape::Optional
        ));

        let tag = shape
            .flags
            .iter()
            .find(|flag| flag.name == "--tag")
            .expect("tag flag exists");
        assert!(tag.repeatable);

        let token = shape
            .flags
            .iter()
            .find(|flag| flag.name == "--token")
            .expect("token flag exists");
        assert!(token.required);
    }

    #[test]
    fn shape_keeps_nested_candidates_from_child_help() {
        let target = target();
        let flow_help = observation(
            "e_000003",
            ProbeIntent::Help,
            vec!["flow".to_owned()],
            "Commands:\n  search  Search flows\n",
            Some(0),
        );

        let shape = infer_shape(target, &[flow_help]);

        assert!(
            shape
                .commands
                .iter()
                .any(|command| command.path == ["flow", "search"])
        );
    }

    #[test]
    fn diagnostic_probes_close_diagnostic_gaps() {
        let target = target();
        let observations = vec![
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["measure".to_owned()],
                "Usage: cliare measure <TARGET>\n\nCommands:\n  nested  Nested command\n\nOptions:\n  --out <DIR>  Output directory\n",
                Some(0),
            ),
            observation(
                "e_000007",
                ProbeIntent::InvalidChild,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
            observation(
                "e_000009",
                ProbeIntent::InvalidFlag,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
        ];

        let shape = infer_shape(target, &observations);
        let measure = shape
            .commands
            .iter()
            .find(|command| command.path == ["measure"])
            .expect("measure command exists");

        assert!(measure.runtime_confirmed);
        assert!(!shape.gaps.iter().any(|gap| {
            gap.command_path == ["measure"]
                && matches!(
                    gap.kind,
                    super::GapKind::InvalidChildDiagnosticsUnknown
                        | super::GapKind::InvalidFlagDiagnosticsUnknown
                )
        }));
    }

    fn target() -> TargetFingerprint {
        TargetFingerprint {
            requested: "cliare".into(),
            resolved: "/tmp/cliare".into(),
            binary_sha256: "abc".to_owned(),
            size_bytes: 1,
        }
    }

    fn observation(
        evidence_id: &str,
        intent: ProbeIntent,
        path: Vec<String>,
        stdout: &str,
        exit_code: Option<i32>,
    ) -> ShapeObservation {
        ShapeObservation {
            evidence_id: evidence_id.to_owned(),
            intent,
            path,
            process: ProcessCompleted {
                probe_id: "p_000001".to_owned(),
                argv: vec!["cliare".to_owned(), "--help".to_owned()],
                status: ProcessStatus::Exited { code: exit_code },
                duration_ms: 1,
                stdout: output(stdout),
                stderr: output(""),
                side_effects: crate::sandbox::SideEffectSummary::default(),
            },
        }
    }

    fn output(text: &str) -> OutputCapture {
        OutputCapture {
            sha256: "unused".to_owned(),
            bytes: text.len(),
            retained_bytes: text.len(),
            truncated: false,
            text: Some(text.to_owned()),
        }
    }
}
