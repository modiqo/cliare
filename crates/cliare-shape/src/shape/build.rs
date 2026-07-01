use crate::claims::{
    ClaimSet, CommandClaim, FlagClaim, FlagValueKind, OutputContractClaim, PositionalClaim,
};
use crate::observation::ShapeObservation;
use cliare_runtime::fingerprint::TargetFingerprint;

use super::gaps::gap_items;
use super::model::{
    CommandCandidate, CommandRuntimeState, CommandShape, FlagCandidate, FlagValueKindShape,
    InferenceModel, OutputContractCandidate, PositionalArgument,
};
use super::{INFERENCE_MODEL, SCHEMA_VERSION};

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

pub(super) fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}

pub(super) fn command_candidate(binary_name: &str, command: &CommandClaim) -> CommandCandidate {
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

pub(super) fn command_runtime_state(command: &CommandClaim) -> CommandRuntimeState {
    if command.runtime_confirmed() {
        CommandRuntimeState::RuntimeConfirmed
    } else if command.precondition_blocked() {
        CommandRuntimeState::PreconditionBlocked
    } else {
        CommandRuntimeState::Unconfirmed
    }
}

pub(super) fn flag_candidate(flag: &FlagClaim) -> FlagCandidate {
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

pub(super) fn positional_argument(argument: &PositionalClaim) -> PositionalArgument {
    PositionalArgument {
        name: argument.name().to_owned(),
        required: argument.required(),
        variadic: argument.variadic(),
        evidence: argument.evidence().to_vec(),
    }
}

pub(super) fn output_contract_candidate(contract: &OutputContractClaim) -> OutputContractCandidate {
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

pub(super) fn flag_value_kind(kind: FlagValueKind) -> FlagValueKindShape {
    match kind {
        FlagValueKind::Boolean => FlagValueKindShape::Boolean,
        FlagValueKind::Required => FlagValueKindShape::Required,
        FlagValueKind::Optional => FlagValueKindShape::Optional,
    }
}

pub(super) fn command_id(binary_name: &str, path: &[String]) -> String {
    let mut id = binary_name.to_owned();
    for segment in path {
        id.push('.');
        id.push_str(segment);
    }
    id
}
