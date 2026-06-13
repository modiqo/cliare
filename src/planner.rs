use std::cmp::Ordering;
use std::collections::{BTreeSet, VecDeque};

use crate::claims::{ClaimSet, CommandClaim, OutputContractClaim};
use crate::evidence::ProbeIntent;
use crate::output::OutputMode;
use crate::process::ProbeSpec;

pub trait ProbePlanner {
    fn seed(&mut self, probes: impl IntoIterator<Item = ProbeSpec>);
    fn extend_from_claims(&mut self, claims: &ClaimSet);
    fn next(&mut self) -> Option<ProbeSpec>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvergencePolicy {
    pub min_expected_value: u16,
}

impl ConvergencePolicy {
    pub fn new(min_expected_value: u16) -> Self {
        Self { min_expected_value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannerStats {
    pub max_depth: usize,
    pub min_expected_value: u16,
    pub frontier_remaining: usize,
    pub highest_pending_expected_value: Option<u16>,
    pub candidates_skipped_by_depth: usize,
    pub candidates_skipped_by_convergence: usize,
}

#[derive(Debug)]
pub struct DeterministicPlanner {
    queue: VecDeque<ProbePlan>,
    scheduled_args: BTreeSet<Vec<String>>,
    depth_skipped_paths: BTreeSet<Vec<String>>,
    convergence_skipped_args: BTreeSet<Vec<String>>,
    max_depth: usize,
    convergence_policy: ConvergencePolicy,
    invalid_token_seed: String,
}

impl DeterministicPlanner {
    pub fn new(max_depth: usize, invalid_token_seed: String) -> Self {
        Self::with_policy(max_depth, ConvergencePolicy::new(0), invalid_token_seed)
    }

    pub fn with_policy(
        max_depth: usize,
        convergence_policy: ConvergencePolicy,
        invalid_token_seed: String,
    ) -> Self {
        Self {
            queue: VecDeque::new(),
            scheduled_args: BTreeSet::new(),
            depth_skipped_paths: BTreeSet::new(),
            convergence_skipped_args: BTreeSet::new(),
            max_depth,
            convergence_policy,
            invalid_token_seed,
        }
    }

    fn schedule(&mut self, plan: ProbePlan) -> bool {
        if plan.expected_value < self.convergence_policy.min_expected_value {
            self.convergence_skipped_args
                .insert(plan.probe.args.clone());
            return false;
        }
        if !self.scheduled_args.insert(plan.probe.args.clone()) {
            return false;
        }

        self.queue.push_back(plan);
        true
    }

    fn schedule_ranked(&mut self, plans: impl IntoIterator<Item = ProbePlan>) {
        let mut plans = plans.into_iter().collect::<Vec<_>>();
        plans.sort();

        for plan in plans {
            self.schedule(plan);
        }
    }

    pub fn stats(&self) -> PlannerStats {
        PlannerStats {
            max_depth: self.max_depth,
            min_expected_value: self.convergence_policy.min_expected_value,
            frontier_remaining: self.queue.len(),
            highest_pending_expected_value: self.queue.iter().map(|plan| plan.expected_value).max(),
            candidates_skipped_by_depth: self.depth_skipped_paths.len(),
            candidates_skipped_by_convergence: self.convergence_skipped_args.len(),
        }
    }

    fn probe_plans_for(&mut self, claim: &CommandClaim) -> Vec<ProbePlan> {
        if claim.path().is_empty() {
            return Vec::new();
        }
        if claim.path().len() > self.max_depth {
            self.depth_skipped_paths.insert(claim.path().to_vec());
            return Vec::new();
        }

        let path = claim.path().to_vec();
        let mut plans = Vec::new();

        if !claim.runtime_confirmed() {
            plans.push(ProbePlan::new(
                ProbeSpec::path_help(path.clone()),
                PlannerRank::for_help_confirmation(claim, 0),
            ));
            plans.push(ProbePlan::new(
                ProbeSpec::help_path(path.clone()),
                PlannerRank::for_help_confirmation(claim, 1),
            ));
        } else {
            if claim.has_child_candidates() && !claim.invalid_child_rejected() {
                plans.push(ProbePlan::new(
                    ProbeSpec::invalid_child(path.clone(), self.invalid_child_token(&path)),
                    PlannerRank::for_diagnostic_probe(claim, 0),
                ));
            }
            if !claim.invalid_flag_rejected() {
                plans.push(ProbePlan::new(
                    ProbeSpec::invalid_flag(path.clone(), self.invalid_flag_token(&path)),
                    PlannerRank::for_diagnostic_probe(claim, 1),
                ));
            }
        }

        plans
    }

    fn output_probe_plans_for(&mut self, claim: &OutputContractClaim) -> Vec<ProbePlan> {
        if claim.command_path().len() > self.max_depth {
            self.depth_skipped_paths
                .insert(claim.command_path().to_vec());
            return Vec::new();
        }
        if claim.probed() {
            return Vec::new();
        }

        let Some(intent) = output_intent(claim.mode()) else {
            return Vec::new();
        };
        vec![ProbePlan::new(
            ProbeSpec::output_mode_help(
                claim.command_path().to_vec(),
                claim.argv_fragment().to_vec(),
                intent,
            ),
            PlannerRank::for_output_probe(claim),
        )]
    }

    fn invalid_child_token(&self, path: &[String]) -> String {
        let suffix = path.join("_").replace('-', "_");
        format!(
            "__cliare_unknown_{}_{}_child__",
            self.invalid_token_seed, suffix
        )
    }

    fn invalid_flag_token(&self, path: &[String]) -> String {
        let suffix = path.join("_").replace('-', "_");
        format!(
            "--__cliare_unknown_{}_{}_flag__",
            self.invalid_token_seed, suffix
        )
    }
}

impl ProbePlanner for DeterministicPlanner {
    fn seed(&mut self, probes: impl IntoIterator<Item = ProbeSpec>) {
        for probe in probes {
            self.schedule(ProbePlan::bootstrap(probe));
        }
    }

    fn extend_from_claims(&mut self, claims: &ClaimSet) {
        let mut plans = Vec::new();
        for claim in claims.commands() {
            plans.extend(self.probe_plans_for(claim));
        }
        for claim in claims.output_contracts() {
            plans.extend(self.output_probe_plans_for(claim));
        }
        self.schedule_ranked(plans);
    }

    fn next(&mut self) -> Option<ProbeSpec> {
        self.queue.pop_front().map(|plan| plan.probe)
    }
}

#[derive(Debug, Clone)]
struct ProbePlan {
    rank: PlannerRank,
    probe: ProbeSpec,
    expected_value: u16,
}

impl ProbePlan {
    fn new(probe: ProbeSpec, rank: PlannerRank) -> Self {
        Self {
            rank,
            probe,
            expected_value: rank.expected_value,
        }
    }

    fn bootstrap(probe: ProbeSpec) -> Self {
        Self {
            rank: PlannerRank::bootstrap(),
            probe,
            expected_value: 1_000,
        }
    }
}

impl Ord for ProbePlan {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank
            .cmp(&other.rank)
            .then_with(|| other.expected_value.cmp(&self.expected_value))
            .then_with(|| self.probe.args.cmp(&other.probe.args))
    }
}

impl PartialOrd for ProbePlan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ProbePlan {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank
            && self.expected_value == other.expected_value
            && self.probe.args == other.probe.args
    }
}

impl Eq for ProbePlan {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlannerRank {
    category: u8,
    expected_value: u16,
    uncertainty: u16,
    confidence: u16,
    depth: u16,
    intent_order: u8,
}

impl PlannerRank {
    fn bootstrap() -> Self {
        Self {
            category: 0,
            expected_value: 1_000,
            uncertainty: 1_000,
            confidence: 1_000,
            depth: 0,
            intent_order: 0,
        }
    }

    fn for_help_confirmation(claim: &CommandClaim, intent_order: u8) -> Self {
        let confidence = quantized_confidence(claim.confidence());
        let uncertainty = uncertainty(confidence);
        Self {
            category: 0,
            expected_value: help_expected_value(confidence, uncertainty),
            uncertainty,
            confidence,
            depth: claim.path().len() as u16,
            intent_order,
        }
    }

    fn for_diagnostic_probe(claim: &CommandClaim, intent_order: u8) -> Self {
        Self {
            category: 1,
            expected_value: diagnostic_expected_value(quantized_confidence(claim.confidence())),
            uncertainty: 0,
            confidence: quantized_confidence(claim.confidence()),
            depth: claim.path().len() as u16,
            intent_order,
        }
    }

    fn for_output_probe(claim: &OutputContractClaim) -> Self {
        Self {
            category: 2,
            expected_value: output_expected_value(claim.mode()),
            uncertainty: if claim.probed() { 0 } else { 700 },
            confidence: if claim.advertised() { 800 } else { 200 },
            depth: claim.command_path().len() as u16,
            intent_order: output_intent_order(claim.mode()),
        }
    }
}

impl Ord for PlannerRank {
    fn cmp(&self, other: &Self) -> Ordering {
        self.category
            .cmp(&other.category)
            .then_with(|| other.expected_value.cmp(&self.expected_value))
            .then_with(|| other.uncertainty.cmp(&self.uncertainty))
            .then_with(|| other.confidence.cmp(&self.confidence))
            .then_with(|| self.depth.cmp(&other.depth))
            .then_with(|| self.intent_order.cmp(&other.intent_order))
    }
}

impl PartialOrd for PlannerRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn quantized_confidence(confidence: f64) -> u16 {
    (confidence.clamp(0.0, 1.0) * 1_000.0).round() as u16
}

fn uncertainty(confidence: u16) -> u16 {
    500_u16.saturating_sub(500_u16.abs_diff(confidence))
}

fn help_expected_value(confidence: u16, uncertainty: u16) -> u16 {
    uncertainty.saturating_add(confidence / 4).min(1_000)
}

fn diagnostic_expected_value(confidence: u16) -> u16 {
    200_u16.saturating_add(confidence / 2).min(1_000)
}

fn output_expected_value(mode: OutputMode) -> u16 {
    match mode {
        OutputMode::Json => 700,
        OutputMode::Yaml => 550,
        OutputMode::Table => 250,
        OutputMode::Plain => 150,
    }
}

fn output_intent(mode: OutputMode) -> Option<ProbeIntent> {
    match mode {
        OutputMode::Json => Some(ProbeIntent::OutputJson),
        OutputMode::Yaml => Some(ProbeIntent::OutputYaml),
        OutputMode::Table => Some(ProbeIntent::OutputTable),
        OutputMode::Plain => Some(ProbeIntent::OutputPlain),
    }
}

fn output_intent_order(mode: OutputMode) -> u8 {
    match mode {
        OutputMode::Json => 0,
        OutputMode::Yaml => 1,
        OutputMode::Table => 2,
        OutputMode::Plain => 3,
    }
}

pub fn bootstrap_invalid_command_token(target_name: &str) -> String {
    format!("__cliare_unknown_{target_name}_command__")
}

pub fn bootstrap_invalid_flag_token(target_name: &str) -> String {
    format!("--__cliare_unknown_{target_name}_flag__")
}

#[cfg(test)]
mod tests {
    use super::{ConvergencePolicy, DeterministicPlanner, ProbePlanner};
    use crate::claims::ClaimSet;
    use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
    use crate::observation::ShapeObservation;
    use crate::process::OutputCapture;

    #[test]
    fn planner_schedules_confirmation_before_diagnostics() {
        let observations = vec![observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  measure  Run probes\n",
            Some(0),
        )];
        let claims = ClaimSet::from_observations("cliare", &observations);
        let mut planner = DeterministicPlanner::new(2, "cliare".to_owned());

        planner.extend_from_claims(&claims);
        let first = planner.next().expect("first probe");

        assert_eq!(first.intent, ProbeIntent::Help);
        assert_eq!(first.args, ["measure", "--help"]);
    }

    #[test]
    fn planner_schedules_diagnostics_for_confirmed_commands() {
        let observations = vec![observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nCommands:\n  child  Nested command\n\nOptions:\n  --out <DIR>  Output directory\n",
            Some(0),
        )];
        let claims = ClaimSet::from_observations("cliare", &observations);
        let mut planner = DeterministicPlanner::new(2, "cliare".to_owned());

        planner.extend_from_claims(&claims);
        let probes = std::iter::from_fn(|| planner.next())
            .map(|probe| probe.intent)
            .collect::<Vec<_>>();

        assert!(probes.contains(&ProbeIntent::InvalidChild));
        assert!(probes.contains(&ProbeIntent::InvalidFlag));
    }

    #[test]
    fn planner_does_not_send_invalid_child_to_leaf_commands() {
        let observations = vec![observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
            Some(0),
        )];
        let claims = ClaimSet::from_observations("cliare", &observations);
        let mut planner = DeterministicPlanner::new(2, "cliare".to_owned());

        planner.extend_from_claims(&claims);
        let probes = std::iter::from_fn(|| planner.next())
            .map(|probe| probe.intent)
            .collect::<Vec<_>>();

        assert!(!probes.contains(&ProbeIntent::InvalidChild));
        assert!(probes.contains(&ProbeIntent::InvalidFlag));
    }

    #[test]
    fn planner_respects_deep_recursion_limit() {
        let observations = vec![
            observation(
                "e_000001",
                ProbeIntent::Help,
                vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()],
                "Usage: tool alpha beta gamma <COMMAND>\n\nCommands:\n  delta  Continue\n",
                Some(0),
            ),
            observation(
                "e_000002",
                ProbeIntent::Help,
                vec![
                    "alpha".to_owned(),
                    "beta".to_owned(),
                    "gamma".to_owned(),
                    "delta".to_owned(),
                ],
                "Usage: tool alpha beta gamma delta <COMMAND>\n\nCommands:\n  epsilon  Continue\n",
                Some(0),
            ),
        ];
        let claims = ClaimSet::from_observations("tool", &observations);

        let mut shallow = DeterministicPlanner::new(3, "tool".to_owned());
        shallow.extend_from_claims(&claims);
        let shallow_probes = std::iter::from_fn(|| shallow.next())
            .map(|probe| probe.args)
            .collect::<Vec<_>>();

        let mut deep = DeterministicPlanner::new(5, "tool".to_owned());
        deep.extend_from_claims(&claims);
        let deep_probes = std::iter::from_fn(|| deep.next())
            .map(|probe| probe.args)
            .collect::<Vec<_>>();

        assert!(!shallow_probes.iter().any(|args| args
            == &[
                "alpha".to_owned(),
                "beta".to_owned(),
                "gamma".to_owned(),
                "delta".to_owned(),
                "epsilon".to_owned(),
                "--help".to_owned(),
            ]));
        assert!(deep_probes.iter().any(|args| args
            == &[
                "alpha".to_owned(),
                "beta".to_owned(),
                "gamma".to_owned(),
                "delta".to_owned(),
                "epsilon".to_owned(),
                "--help".to_owned(),
            ]));
    }

    #[test]
    fn planner_skips_low_value_dynamic_probes_by_convergence_policy() {
        let observations = vec![observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  maybe  Weak candidate\n",
            Some(0),
        )];
        let claims = ClaimSet::from_observations("tool", &observations);
        let mut planner =
            DeterministicPlanner::with_policy(2, ConvergencePolicy::new(1_000), "tool".to_owned());

        planner.extend_from_claims(&claims);

        assert!(planner.next().is_none());
        let stats = planner.stats();
        assert_eq!(stats.frontier_remaining, 0);
        assert_eq!(stats.candidates_skipped_by_convergence, 2);
        assert_eq!(stats.min_expected_value, 1_000);
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
