use crate::claims::CommandClaim;
use cliare_inference::output::ObservedOutputKind;

use super::model::{FlagCandidate, FlagValueKindShape, Gap, GapKind, OutputContractCandidate};

pub(super) fn gap_items<'a>(
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
        if command.alternate_help_unavailable() {
            gaps.push(Gap {
                kind: GapKind::AlternateHelpFormUnavailable,
                command_path: command.path().to_vec(),
                reason: "optional `help <command path>` probe did not resolve this command"
                    .to_owned(),
                evidence: command.alternate_help_evidence().to_vec(),
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
