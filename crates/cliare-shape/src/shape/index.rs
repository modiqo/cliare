use crate::observation::ShapeObservation;
use cliare_inference::output::ObservedOutputKind;
use cliare_inference::precondition::PreconditionKind;
use cliare_runtime::fingerprint::TargetFingerprint;

use super::build::infer_shape;
use super::labels::{command_display, gap_kind_label};
use super::model::{
    AgentSuitability, CommandCandidate, CommandIndex, CommandIndexEntry, CommandIndexFlag,
    CommandIndexGap, CommandIndexOutputContract, CommandIndexSummary, CommandParameters,
    CommandRuntimeState, CommandShape, FlagCandidate, Gap, GapKind, InferenceModel,
    OutputContractCandidate, OutputContractStatus,
};
use super::{COMMAND_INDEX_SCHEMA_VERSION, INFERENCE_MODEL, SCHEMA_VERSION};

pub fn infer_command_index(
    target: TargetFingerprint,
    observations: &[ShapeObservation],
) -> CommandIndex {
    let shape = infer_shape(target, observations);
    command_index(&shape)
}

pub(super) fn command_index(shape: &CommandShape) -> CommandIndex {
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

pub(super) fn command_index_entry(
    command: &CommandCandidate,
    shape: &CommandShape,
) -> CommandIndexEntry {
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

pub(super) fn command_index_summary(commands: &[CommandIndexEntry]) -> CommandIndexSummary {
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

pub(super) fn command_index_flag(flag: &FlagCandidate) -> CommandIndexFlag {
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

pub(super) fn command_index_output_contract(
    contract: &OutputContractCandidate,
) -> CommandIndexOutputContract {
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

pub(super) fn output_contract_status(contract: &OutputContractCandidate) -> OutputContractStatus {
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

pub(super) fn command_index_gap(gap: &Gap) -> CommandIndexGap {
    CommandIndexGap {
        kind: gap.kind,
        reason: gap.reason.clone(),
        evidence: gap.evidence.clone(),
    }
}

pub(super) fn command_preconditions(
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

pub(super) fn command_suitability(
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

pub(super) fn preconditions_are_agent_satisfiable(preconditions: &[PreconditionKind]) -> bool {
    !preconditions.is_empty()
        && preconditions.iter().all(|precondition| {
            matches!(
                precondition,
                PreconditionKind::AuthRequired | PreconditionKind::LocalContextRequired
            )
        })
}

pub(super) fn format_preconditions(preconditions: &[PreconditionKind]) -> String {
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
