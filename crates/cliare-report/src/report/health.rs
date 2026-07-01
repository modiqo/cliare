use super::*;

pub(super) fn command_health(command_index: &CommandIndexArtifact) -> Vec<CommandHealth> {
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
