use std::collections::{BTreeMap, BTreeSet};

use cliare_core::process_status::ProcessStatus;
use cliare_evidence::ProbeIntent;
use cliare_inference::layout;
use cliare_shape::claims::{ClaimSet, CommandClaim};
use cliare_shape::observation::ShapeObservation;

use super::metrics::{Metrics, process_text};
use super::model::{
    AgentNavigation, AgentNavigationCapability, AgentNavigationMetric, AgentNavigationMetricStatus,
};
use super::util::{ratio, round_score};

const STATUS_EXPERIMENTAL: &str = "experimental";
const EVIDENCE_LIMIT: usize = 8;

pub(super) fn agent_navigation(
    claims: &ClaimSet,
    binary_name: &str,
    observations: &[ShapeObservation],
    metrics: &Metrics,
) -> AgentNavigation {
    let commands = claims.commands().collect::<Vec<_>>();
    let mut dimensions = BTreeMap::new();

    dimensions.insert(
        AgentNavigationCapability::CanonicalHelpCoverage,
        canonical_help_coverage(&commands, binary_name, observations),
    );
    dimensions.insert(
        AgentNavigationCapability::UsageCoverage,
        usage_coverage(&commands),
    );
    dimensions.insert(
        AgentNavigationCapability::SubcommandTableClarity,
        subcommand_table_clarity(claims, binary_name, observations),
    );
    dimensions.insert(
        AgentNavigationCapability::PositionalOperandCoverage,
        positional_operand_coverage(&commands),
    );
    dimensions.insert(
        AgentNavigationCapability::OutputContractParseCoverage,
        output_contract_parse_coverage(metrics),
    );
    dimensions.insert(
        AgentNavigationCapability::InvalidInputRecovery,
        invalid_input_recovery(metrics, observations),
    );
    dimensions.insert(
        AgentNavigationCapability::DiscoverySideEffectSafety,
        discovery_side_effect_safety(metrics, observations),
    );
    dimensions.insert(
        AgentNavigationCapability::PreconditionClarity,
        precondition_clarity(metrics, observations),
    );
    dimensions.insert(
        AgentNavigationCapability::ExampleValidity,
        example_validity(),
    );

    AgentNavigation {
        status: STATUS_EXPERIMENTAL,
        dimensions,
        limitations: navigation_limitations(metrics),
    }
}

fn canonical_help_coverage(
    commands: &[&CommandClaim],
    binary_name: &str,
    observations: &[ShapeObservation],
) -> AgentNavigationMetric {
    if commands.is_empty() {
        return no_evidence(
            "No command candidates were discovered, so command-specific canonical help coverage has no measured denominator.",
            Vec::new(),
            vec!["Command recall requires discovered candidates or an external truth set."],
        );
    }

    let canonical_help = observations
        .iter()
        .filter(|observation| canonical_help_match(observation, binary_name))
        .map(|observation| (observation.path.clone(), observation.evidence_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let numerator = commands
        .iter()
        .filter(|command| canonical_help.contains_key(command.path().as_slice()))
        .count();
    let evidence = canonical_help
        .values()
        .take(EVIDENCE_LIMIT)
        .cloned()
        .collect::<Vec<_>>();

    measured(
        numerator,
        commands.len(),
        "Commands with path-matching direct `<command> --help` or `-h` evidence divided by discovered commands.",
        evidence,
        vec![
            "This is bounded by traversal: undiscovered commands are not in the denominator.",
            "Direct help-like bare group output is not yet counted as canonical help evidence.",
        ],
    )
}

fn usage_coverage(commands: &[&CommandClaim]) -> AgentNavigationMetric {
    if commands.is_empty() {
        return no_evidence(
            "No command candidates were discovered, so usage coverage has no measured denominator.",
            Vec::new(),
            vec!["Command recall requires discovered candidates or an external truth set."],
        );
    }

    let commands_with_usage = commands
        .iter()
        .copied()
        .filter(|command| command.usage_observed())
        .collect::<Vec<_>>();
    measured(
        commands_with_usage.len(),
        commands.len(),
        "Discovered commands with matching usage syntax divided by discovered commands.",
        command_evidence(&commands_with_usage),
        vec!["This does not yet validate release-to-release usage stability."],
    )
}

fn subcommand_table_clarity(
    claims: &ClaimSet,
    binary_name: &str,
    observations: &[ShapeObservation],
) -> AgentNavigationMetric {
    let group_paths = claims
        .commands()
        .filter(|command| command.has_child_candidates())
        .map(|command| command.path().to_vec())
        .collect::<BTreeSet<_>>();

    let mut denominator = 0_usize;
    let mut numerator = 0_usize;
    let mut evidence = Vec::new();

    for observation in observations {
        if observation.intent != ProbeIntent::Help || !exited_zero(&observation.process.status) {
            continue;
        }
        let Some(text) = process_text(&observation.process) else {
            continue;
        };
        let profile = layout::extraction_profile(text, binary_name, &observation.path);
        let relevant_group_help = observation.path.is_empty()
            || group_paths.contains(&observation.path)
            || profile.command_candidates > 0;
        if !profile.help_like || !relevant_group_help {
            continue;
        }

        denominator += 1;
        if profile.command_candidates > 0 {
            numerator += 1;
            push_limited(&mut evidence, observation.evidence_id.clone());
        }
    }

    if denominator == 0 {
        return no_evidence(
            "No help-like root or command-group output was observed for subcommand-table clarity.",
            Vec::new(),
            vec![
                "Leaf command help without children is intentionally excluded.",
                "If traversal missed parent groups, this denominator is incomplete.",
            ],
        );
    }

    measured(
        numerator,
        denominator,
        "Help-like root or command-group observations with extracted command candidates divided by relevant group-help observations.",
        evidence,
        vec![
            "This measures parseable command-table evidence, not complete command recall.",
            "Command groups hidden behind prose or direct bare invocation are not fully represented yet.",
        ],
    )
}

fn positional_operand_coverage(commands: &[&CommandClaim]) -> AgentNavigationMetric {
    let recognized = commands
        .iter()
        .copied()
        .filter(|command| command.runtime_confirmed() || command.precondition_blocked())
        .collect::<Vec<_>>();
    if recognized.is_empty() {
        return no_evidence(
            "No runtime-recognized commands were observed, so positional operand coverage has no measured denominator.",
            Vec::new(),
            vec![
                "Candidate-only commands may still contain operands in command rows, but those are not first-class positional claims yet.",
            ],
        );
    }

    let with_usage = recognized
        .iter()
        .copied()
        .filter(|command| command.usage_observed())
        .collect::<Vec<_>>();
    measured(
        with_usage.len(),
        recognized.len(),
        "Runtime-recognized commands with usage syntax divided by runtime-recognized commands.",
        command_evidence(&with_usage),
        vec![
            "Current implementation uses matching usage syntax as the strongest proxy for positional arity.",
            "Command-table operands and missing-argument diagnostics are not yet promoted to full positional claims.",
        ],
    )
}

fn output_contract_parse_coverage(metrics: &Metrics) -> AgentNavigationMetric {
    if metrics.machine_readable_output_contracts == 0 {
        return no_evidence_with_zero_score(
            "No machine-readable output contracts were discovered; agents have no measured parseable result contract.",
            Vec::new(),
            vec![
                "Commands may still print structured human text, but CLIARE has no JSON/YAML contract evidence.",
            ],
        );
    }

    measured(
        metrics.output_mode_parse_successes,
        metrics.machine_readable_output_contracts,
        "Machine-readable output contracts with successful parse probes divided by discovered machine-readable output contracts.",
        Vec::new(),
        vec!["Contracts blocked by missing fixtures remain unvalidated until safe operands exist."],
    )
}

fn invalid_input_recovery(
    metrics: &Metrics,
    observations: &[ShapeObservation],
) -> AgentNavigationMetric {
    if metrics.invalid_probe_count == 0 {
        return no_evidence(
            "No invalid-input diagnostic probes were measured.",
            Vec::new(),
            vec!["Recovery quality needs invalid command, child, or flag probes."],
        );
    }

    let evidence = observations
        .iter()
        .filter(|observation| {
            matches!(
                observation.intent,
                ProbeIntent::InvalidCommand | ProbeIntent::InvalidChild | ProbeIntent::InvalidFlag
            )
        })
        .map(|observation| observation.evidence_id.clone())
        .take(EVIDENCE_LIMIT)
        .collect::<Vec<_>>();

    measured(
        metrics.invalid_probe_rejections + metrics.invalid_probe_actionable,
        metrics.invalid_probe_count.saturating_mul(2),
        "Invalid-input probes receive half credit for nonzero rejection and half credit for actionable recovery text.",
        evidence,
        vec![
            "Actionability is currently detected from labeled fix/hint blocks and command examples.",
        ],
    )
}

fn discovery_side_effect_safety(
    metrics: &Metrics,
    observations: &[ShapeObservation],
) -> AgentNavigationMetric {
    if !metrics.side_effect_observation_supported() {
        return not_measured(
            "Side-effect safety is not measured in host execution mode.",
            side_effect_evidence(observations),
            vec!["Run in isolated mode to score safe-probe side-effect behavior."],
        );
    }
    if metrics.coverage.probes_completed == 0 {
        return no_evidence(
            "No completed probes were observed, so discovery side-effect safety has no measured denominator.",
            Vec::new(),
            Vec::new(),
        );
    }

    let denominator = metrics
        .coverage
        .probes_completed
        .saturating_add(metrics.credential_like_side_effects);
    let numerator = metrics
        .coverage
        .probes_completed
        .saturating_sub(metrics.side_effect_probe_count);

    measured(
        numerator,
        denominator.max(1),
        "Completed probes without persistent side effects, additionally penalized for credential-like side-effect paths.",
        side_effect_evidence(observations),
        vec![
            "This covers filesystem side effects in registered sandbox regions, not network behavior.",
        ],
    )
}

fn precondition_clarity(
    metrics: &Metrics,
    observations: &[ShapeObservation],
) -> AgentNavigationMetric {
    if metrics.coverage.precondition_blocked_probes == 0 {
        return no_evidence(
            "No precondition-blocked probes were observed, so precondition diagnostic clarity had no opportunity to score.",
            Vec::new(),
            Vec::new(),
        );
    }

    measured(
        metrics.coverage.actionable_precondition_probes,
        metrics.coverage.precondition_blocked_probes,
        "Precondition-blocked probes with actionable recovery diagnostics divided by precondition-blocked probes.",
        precondition_evidence(observations),
        vec!["Actionability requires a concrete next step, fix block, or command example."],
    )
}

fn example_validity() -> AgentNavigationMetric {
    not_measured(
        "Example syntax validity is not measured yet.",
        Vec::new(),
        vec![
            "Examples are currently weak hints for output modes and diagnostics.",
            "Future work should parse examples, match them to known command paths, and validate syntax without unsafe execution.",
        ],
    )
}

fn canonical_help_match(observation: &ShapeObservation, binary_name: &str) -> bool {
    if observation.intent != ProbeIntent::Help
        || observation.path.is_empty()
        || alternate_help_invocation(observation)
        || !direct_help_invocation(observation)
        || !exited_zero(&observation.process.status)
    {
        return false;
    }
    let Some(text) = observation.process.stdout.text.as_deref() else {
        return false;
    };
    layout::is_help_like(text)
        && layout::help_matches_command_path(text, binary_name, &observation.path)
}

fn alternate_help_invocation(observation: &ShapeObservation) -> bool {
    observation
        .process
        .argv
        .get(1)
        .is_some_and(|arg| arg == "help")
}

fn direct_help_invocation(observation: &ShapeObservation) -> bool {
    let args = observation.process.argv.get(1..).unwrap_or_default();
    let Some((help_arg, command_args)) = args.split_last() else {
        return false;
    };
    matches!(help_arg.as_str(), "--help" | "-h") && command_args == observation.path.as_slice()
}

fn measured(
    numerator: usize,
    denominator: usize,
    rationale: &str,
    evidence: Vec<String>,
    limitations: Vec<&str>,
) -> AgentNavigationMetric {
    AgentNavigationMetric {
        score: Some(round_score(100.0 * ratio(numerator, denominator))),
        numerator,
        denominator,
        status: AgentNavigationMetricStatus::Measured,
        rationale: rationale.to_owned(),
        evidence: limited_unique(evidence),
        limitations: limitations.into_iter().map(str::to_owned).collect(),
    }
}

fn no_evidence(
    rationale: &str,
    evidence: Vec<String>,
    limitations: Vec<&str>,
) -> AgentNavigationMetric {
    AgentNavigationMetric {
        score: None,
        numerator: 0,
        denominator: 0,
        status: AgentNavigationMetricStatus::NoEvidence,
        rationale: rationale.to_owned(),
        evidence: limited_unique(evidence),
        limitations: limitations.into_iter().map(str::to_owned).collect(),
    }
}

fn no_evidence_with_zero_score(
    rationale: &str,
    evidence: Vec<String>,
    limitations: Vec<&str>,
) -> AgentNavigationMetric {
    AgentNavigationMetric {
        score: Some(0.0),
        numerator: 0,
        denominator: 0,
        status: AgentNavigationMetricStatus::NoEvidence,
        rationale: rationale.to_owned(),
        evidence: limited_unique(evidence),
        limitations: limitations.into_iter().map(str::to_owned).collect(),
    }
}

fn not_measured(
    rationale: &str,
    evidence: Vec<String>,
    limitations: Vec<&str>,
) -> AgentNavigationMetric {
    AgentNavigationMetric {
        score: None,
        numerator: 0,
        denominator: 0,
        status: AgentNavigationMetricStatus::NotMeasured,
        rationale: rationale.to_owned(),
        evidence: limited_unique(evidence),
        limitations: limitations.into_iter().map(str::to_owned).collect(),
    }
}

fn navigation_limitations(metrics: &Metrics) -> Vec<String> {
    let mut limitations = vec![
        "Agent navigation metrics are experimental and derived from the same bounded runtime evidence as the scorecard.".to_owned(),
        "They do not prove complete command recall without external truth sets.".to_owned(),
    ];
    if !metrics.coverage.traversal_complete {
        limitations.push(format!(
            "Traversal is incomplete: stop reason `{}` with {} frontier item(s) remaining.",
            traversal_reason_label(metrics.coverage.traversal_stop_reason),
            metrics.coverage.frontier_remaining
        ));
    }
    if !metrics.side_effect_observation_supported() {
        limitations.push(
            "Host execution mode does not support isolated side-effect confidence.".to_owned(),
        );
    }
    limitations
}

fn traversal_reason_label(reason: super::model::TraversalStopReason) -> &'static str {
    match reason {
        super::model::TraversalStopReason::FrontierExhausted => "frontier_exhausted",
        super::model::TraversalStopReason::Converged => "converged",
        super::model::TraversalStopReason::DepthBudgetExhausted => "depth_budget_exhausted",
        super::model::TraversalStopReason::ProbeBudgetExhausted => "probe_budget_exhausted",
    }
}

fn command_evidence(commands: &[&CommandClaim]) -> Vec<String> {
    commands
        .iter()
        .flat_map(|command| command.evidence().iter().cloned())
        .take(EVIDENCE_LIMIT)
        .collect()
}

fn side_effect_evidence(observations: &[ShapeObservation]) -> Vec<String> {
    observations
        .iter()
        .filter(|observation| observation.process.side_effects.total > 0)
        .map(|observation| observation.evidence_id.clone())
        .take(EVIDENCE_LIMIT)
        .collect()
}

fn precondition_evidence(observations: &[ShapeObservation]) -> Vec<String> {
    observations
        .iter()
        .filter(|observation| !exited_zero(&observation.process.status))
        .filter(|observation| {
            cliare_inference::precondition::classify_process(
                &observation.process.status,
                observation.process.stdout.text.as_deref(),
                observation.process.stderr.text.as_deref(),
            )
            .is_some()
        })
        .map(|observation| observation.evidence_id.clone())
        .take(EVIDENCE_LIMIT)
        .collect()
}

fn limited_unique(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            unique.push(value);
        }
        if unique.len() >= EVIDENCE_LIMIT {
            break;
        }
    }
    unique
}

fn push_limited(values: &mut Vec<String>, value: String) {
    if values.len() < EVIDENCE_LIMIT && !values.contains(&value) {
        values.push(value);
    }
}

fn exited_zero(status: &ProcessStatus) -> bool {
    matches!(status, ProcessStatus::Exited { code: Some(0) })
}
