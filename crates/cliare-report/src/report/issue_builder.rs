use std::collections::BTreeMap;

use super::issue_evidence::{issue_command_rank, issue_evidence, issue_evidence_references};
use super::*;
use crate::report_format::{output_mode_label, shell_arg, shell_words};

pub(super) fn issue_from_action_item(
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
