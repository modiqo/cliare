use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::claims::{ClaimSet, CommandClaim, OutputContractClaim};
use crate::process::ProbeSpec;

use super::model::{ConvergencePolicy, PlannerStats, ProbePlanner};
use super::rank::{PlannerRank, ProbePlan, output_help_intent, output_intent};

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

    pub fn mark_seen(&mut self, probes: impl IntoIterator<Item = ProbeSpec>) {
        for probe in probes {
            self.scheduled_args.insert(probe.args);
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

    fn output_probe_plans_for(
        &mut self,
        claim: &OutputContractClaim,
        command: Option<&CommandClaim>,
    ) -> Vec<ProbePlan> {
        if claim.command_path().len() > self.max_depth {
            self.depth_skipped_paths
                .insert(claim.command_path().to_vec());
            return Vec::new();
        }
        if command.is_some_and(has_required_positionals) {
            return Vec::new();
        }

        let Some(intent) = output_intent(claim.mode()) else {
            return Vec::new();
        };
        let Some(help_intent) = output_help_intent(claim.mode()) else {
            return Vec::new();
        };

        let mut plans = Vec::new();
        if !claim.probed() {
            plans.push(ProbePlan::new(
                ProbeSpec::output_mode(
                    claim.command_path().to_vec(),
                    claim.argv_fragment().to_vec(),
                    intent,
                ),
                PlannerRank::for_output_probe(claim, 0),
            ));
        }
        if !claim.help_probed() {
            plans.push(ProbePlan::new(
                ProbeSpec::output_mode_help(
                    claim.command_path().to_vec(),
                    claim.argv_fragment().to_vec(),
                    help_intent,
                ),
                PlannerRank::for_output_probe(claim, 1),
            ));
        }
        plans
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
        let commands = claims
            .commands()
            .map(|claim| (claim.path().to_vec(), claim))
            .collect::<BTreeMap<_, _>>();
        for claim in claims.commands() {
            plans.extend(self.probe_plans_for(claim));
        }
        for claim in claims.output_contracts() {
            plans.extend(self.output_probe_plans_for(
                claim,
                commands.get(claim.command_path().as_slice()).copied(),
            ));
        }
        self.schedule_ranked(plans);
    }

    fn next(&mut self) -> Option<ProbeSpec> {
        self.queue.pop_front().map(|plan| plan.probe)
    }
}

pub fn bootstrap_invalid_command_token(target_name: &str) -> String {
    format!("__cliare_unknown_{target_name}_command__")
}

pub fn bootstrap_invalid_flag_token(target_name: &str) -> String {
    format!("--__cliare_unknown_{target_name}_flag__")
}

fn has_required_positionals(claim: &CommandClaim) -> bool {
    claim.positionals().any(|argument| argument.required())
}
