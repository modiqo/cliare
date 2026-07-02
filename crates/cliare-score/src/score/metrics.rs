use cliare_evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
use cliare_inference::diagnostic::{self, RecoveryQuality};
use cliare_inference::layout;
use cliare_inference::output::{ObservedOutputKind, OutputMode};
use cliare_inference::precondition::PreconditionKind;
use cliare_inference::score_model::ScoreModelSpec;
use cliare_shape::claims::{
    ClaimSet, CommandClaim, FlagClaim, FlagValueKind, OutputContractClaim, OutputContractScope,
};
use cliare_shape::observation::ShapeObservation;

use cliare_policy::path_classification;

use super::model::{Coverage, ScoreRunContext, TraversalStopReason};
use super::util::{average, ratio};

#[derive(Debug)]
pub(super) struct Metrics {
    pub(super) coverage: Coverage,
    pub(super) grammar_gap_count: usize,
    pub(super) flags_with_known_grammar: usize,
    pub(super) machine_readable_output_contracts: usize,
    pub(super) output_mode_scored_contracts: usize,
    pub(super) output_mode_probe_count: usize,
    pub(super) output_mode_parse_successes: usize,
    pub(super) output_mode_precondition_blocked: usize,
    pub(super) output_mode_help_text_probes: usize,
    pub(super) output_mode_global_scope_failures: usize,
    pub(super) side_effect_files_total: usize,
    pub(super) side_effect_probe_count: usize,
    pub(super) credential_like_side_effects: usize,
    pub(super) invalid_probe_count: usize,
    pub(super) invalid_probe_rejections: usize,
    pub(super) invalid_probe_actionable: usize,
    pub(super) extraction: ExtractionMetrics,
}

impl Metrics {
    pub(super) fn from_claims_and_observations(
        claims: &ClaimSet,
        binary_name: &str,
        observations: &[ShapeObservation],
        run_context: ScoreRunContext,
    ) -> Self {
        let commands = claims.commands().collect::<Vec<_>>();
        let flags = claims.flags().collect::<Vec<_>>();
        let outputs = claims.output_contracts().collect::<Vec<_>>();
        let commands_discovered = commands.len();
        let commands_runtime_confirmed = commands
            .iter()
            .filter(|command| command.runtime_confirmed())
            .count();
        let commands_precondition_blocked = commands
            .iter()
            .filter(|command| command.precondition_blocked())
            .count();
        let avg_command_confidence = average(commands.iter().map(|command| command.confidence()));
        let avg_flag_confidence = average(flags.iter().map(|flag| flag.confidence()));
        let grammar_gap_count = commands
            .iter()
            .filter(|command| command.runtime_confirmed())
            .map(|command| grammar_gaps_for(command))
            .sum();
        let flags_with_known_grammar = flags.iter().filter(|flag| flag_grammar_known(flag)).count();
        let output_contracts_discovered = outputs.len();
        let machine_readable_output_contracts = outputs
            .iter()
            .filter(|contract| machine_readable_output_contract(contract))
            .count();
        let output_mode_scored_contracts = outputs
            .iter()
            .filter(|contract| {
                machine_readable_output_contract(contract)
                    && !contract.precondition_blocked()
                    && contract.observed_kind() != Some(ObservedOutputKind::HelpText)
                    && (!contract.scope().is_global_only() || contract.parse_success())
            })
            .count();
        let output_mode_probe_count = outputs.iter().filter(|contract| contract.probed()).count();
        let output_mode_parse_successes = outputs
            .iter()
            .filter(|contract| contract.parse_success())
            .count();
        let output_mode_precondition_blocked = outputs
            .iter()
            .filter(|contract| contract.precondition_blocked())
            .count();
        let output_mode_help_text_probes = outputs
            .iter()
            .filter(|contract| contract.observed_kind() == Some(ObservedOutputKind::HelpText))
            .count();
        let output_mode_global_scope_failures = outputs
            .iter()
            .filter(|contract| {
                contract.probed()
                    && !contract.parse_success()
                    && !contract.precondition_blocked()
                    && contract.observed_kind() != Some(ObservedOutputKind::HelpText)
                    && contract.scope() == OutputContractScope::GlobalFlag
            })
            .count();

        let process_metrics = ProcessMetrics::from_observations(observations);
        let extraction = ExtractionMetrics::from_observations(binary_name, observations);
        let probes_skipped_by_budget =
            if run_context.max_probes > 0 && observations.len() >= run_context.max_probes {
                run_context.frontier_remaining
            } else {
                0
            };
        let traversal_stop_reason = traversal_stop_reason(
            commands_discovered,
            probes_skipped_by_budget,
            run_context.candidates_skipped_by_depth,
            run_context.candidates_skipped_by_convergence,
        );

        Self {
            coverage: Coverage {
                sandbox_profile: run_context.sandbox.profile,
                sandbox_root: run_context.sandbox.root,
                sandbox_home: run_context.sandbox.home,
                sandbox_workdir: run_context.sandbox.workdir,
                sandbox_env_policy: run_context.sandbox.env_policy,
                snapshot_max_files: run_context.sandbox.snapshot_limits.max_files,
                snapshot_max_directories: run_context.sandbox.snapshot_limits.max_directories,
                snapshot_max_hash_bytes: run_context.sandbox.snapshot_limits.max_hash_bytes,
                hostile_binary_containment: run_context.sandbox.hostile_binary_containment,
                commands_discovered,
                commands_runtime_confirmed,
                commands_precondition_blocked,
                command_confirmation_rate: ratio(commands_runtime_confirmed, commands_discovered),
                help_text_probes: extraction.help_text_probes,
                help_text_probes_with_shape: extraction.help_text_probes_with_shape,
                help_text_probes_without_shape: extraction.help_text_probes_without_shape,
                help_text_probes_not_recognized: extraction.help_text_probes_not_recognized,
                parser_extraction_rate: extraction.extraction_rate(),
                flags_discovered: flags.len(),
                output_contracts_discovered,
                machine_readable_output_contracts,
                output_mode_probes_completed: output_mode_probe_count,
                output_mode_parse_successes,
                output_mode_precondition_blocked,
                output_mode_help_text_probes,
                side_effect_files_created: process_metrics.side_effect_files_created,
                side_effect_files_modified: process_metrics.side_effect_files_modified,
                side_effect_files_deleted: process_metrics.side_effect_files_deleted,
                side_effect_files_total: process_metrics.side_effect_files_total,
                side_effect_probe_count: process_metrics.side_effect_probe_count,
                credential_like_side_effects: process_metrics.credential_like_side_effects,
                side_effect_scan_truncated: process_metrics.side_effect_scan_truncated,
                avg_command_confidence,
                avg_flag_confidence,
                observed_max_depth: observed_max_depth(&commands),
                traversal_profile: run_context.traversal_profile,
                max_depth: run_context.max_depth,
                max_probes: run_context.max_probes,
                min_expected_value: run_context.min_expected_value,
                concurrency_limit: run_context.concurrency_limit,
                traversal_rounds: run_context.traversal_rounds,
                probes_scheduled: run_context.probes_scheduled,
                probes_completed: observations.len(),
                probes_cancelled: run_context.probes_cancelled,
                probes_timed_out: process_metrics.timed_out,
                probes_failed_to_spawn: process_metrics.failed_to_spawn,
                precondition_blocked_probes: process_metrics.precondition_blocked,
                auth_required_probes: process_metrics.auth_required,
                local_context_required_probes: process_metrics.local_context_required,
                fixture_required_probes: process_metrics.fixture_required,
                actionable_precondition_probes: process_metrics.actionable_precondition,
                precondition_recovery_rate: ratio(
                    process_metrics.actionable_precondition,
                    process_metrics.precondition_blocked,
                ),
                frontier_remaining: run_context.frontier_remaining,
                highest_pending_expected_value: run_context.highest_pending_expected_value,
                candidates_skipped_by_depth: run_context.candidates_skipped_by_depth,
                candidates_skipped_by_convergence: run_context.candidates_skipped_by_convergence,
                probes_skipped_by_budget,
                budget_exhausted: probes_skipped_by_budget > 0,
                traversal_stop_reason,
                traversal_complete: matches!(
                    traversal_stop_reason,
                    TraversalStopReason::Converged | TraversalStopReason::FrontierExhausted
                ),
            },
            grammar_gap_count,
            flags_with_known_grammar,
            machine_readable_output_contracts,
            output_mode_scored_contracts,
            output_mode_probe_count,
            output_mode_parse_successes,
            output_mode_precondition_blocked,
            output_mode_help_text_probes,
            output_mode_global_scope_failures,
            side_effect_files_total: process_metrics.side_effect_files_total,
            side_effect_probe_count: process_metrics.side_effect_probe_count,
            credential_like_side_effects: process_metrics.credential_like_side_effects,
            invalid_probe_count: process_metrics.invalid_probe_count,
            invalid_probe_rejections: process_metrics.invalid_probe_rejections,
            invalid_probe_actionable: process_metrics.invalid_probe_actionable,
            extraction,
        }
    }

    pub(super) fn grammar_gap_rate(&self) -> f64 {
        let possible = self.coverage.commands_runtime_confirmed.saturating_mul(2);
        ratio(self.grammar_gap_count, possible)
    }

    pub(super) fn flag_grammar_rate(&self) -> f64 {
        ratio(
            self.flags_with_known_grammar,
            self.coverage.flags_discovered,
        )
    }

    pub(super) fn command_recognition_rate(&self) -> f64 {
        ratio(
            self.coverage.commands_runtime_confirmed + self.coverage.commands_precondition_blocked,
            self.coverage.commands_discovered,
        )
    }

    pub(super) fn output_mode_parse_failures(&self) -> usize {
        self.output_mode_probe_count
            .saturating_sub(self.output_mode_parse_successes)
            .saturating_sub(self.output_mode_precondition_blocked)
            .saturating_sub(self.output_mode_help_text_probes)
            .saturating_sub(self.output_mode_global_scope_failures)
    }

    pub(super) fn side_effect_observation_supported(&self) -> bool {
        self.coverage.sandbox_profile != "host"
    }
}

pub(super) fn machine_readable_output_contract(contract: &OutputContractClaim) -> bool {
    matches!(contract.mode(), OutputMode::Json | OutputMode::Yaml)
}

pub(super) fn traversal_stop_reason(
    commands_discovered: usize,
    probes_skipped_by_budget: usize,
    candidates_skipped_by_depth: usize,
    candidates_skipped_by_convergence: usize,
) -> TraversalStopReason {
    if probes_skipped_by_budget > 0 {
        TraversalStopReason::ProbeBudgetExhausted
    } else if candidates_skipped_by_depth > 0 {
        TraversalStopReason::DepthBudgetExhausted
    } else if commands_discovered > 0 || candidates_skipped_by_convergence > 0 {
        TraversalStopReason::Converged
    } else {
        TraversalStopReason::FrontierExhausted
    }
}

pub(super) fn observed_max_depth(commands: &[&CommandClaim]) -> usize {
    commands
        .iter()
        .map(|command| command.path().len())
        .max()
        .unwrap_or(0)
}

#[derive(Debug, Default)]
struct ProcessMetrics {
    pub(super) timed_out: usize,
    pub(super) failed_to_spawn: usize,
    pub(super) side_effect_files_created: usize,
    pub(super) side_effect_files_modified: usize,
    pub(super) side_effect_files_deleted: usize,
    pub(super) side_effect_files_total: usize,
    pub(super) side_effect_probe_count: usize,
    pub(super) credential_like_side_effects: usize,
    pub(super) side_effect_scan_truncated: bool,
    pub(super) precondition_blocked: usize,
    pub(super) auth_required: usize,
    pub(super) local_context_required: usize,
    pub(super) fixture_required: usize,
    pub(super) actionable_precondition: usize,
    pub(super) invalid_probe_count: usize,
    pub(super) invalid_probe_rejections: usize,
    pub(super) invalid_probe_actionable: usize,
}

#[derive(Debug, Default)]
pub(super) struct ExtractionMetrics {
    pub(super) help_text_probes: usize,
    pub(super) help_text_probes_with_shape: usize,
    pub(super) help_text_probes_without_shape: usize,
    pub(super) help_text_probes_not_recognized: usize,
}

impl ExtractionMetrics {
    fn from_observations(binary_name: &str, observations: &[ShapeObservation]) -> Self {
        let mut metrics = Self::default();

        for observation in observations {
            if observation.intent != ProbeIntent::Help || !exited_zero(&observation.process.status)
            {
                continue;
            }

            let Some(text) = process_text(&observation.process) else {
                continue;
            };

            metrics.help_text_probes += 1;
            let profile = layout::extraction_profile(text, binary_name, &observation.path);
            if profile.help_like && profile.has_shape_signal() {
                metrics.help_text_probes_with_shape += 1;
            } else if profile.help_like {
                metrics.help_text_probes_without_shape += 1;
            } else {
                metrics.help_text_probes_not_recognized += 1;
            }
        }

        metrics
    }

    fn extraction_rate(&self) -> f64 {
        ratio(self.help_text_probes_with_shape, self.help_text_probes)
    }

    pub(super) fn measurement_limited(&self, model: &ScoreModelSpec) -> bool {
        self.help_text_probes >= model.thresholds.extraction_limited_min_help_probes
            && self.help_text_probes_with_shape == 0
            && (self.help_text_probes_without_shape > 0 || self.help_text_probes_not_recognized > 0)
    }
}

impl ProcessMetrics {
    fn from_observations(observations: &[ShapeObservation]) -> Self {
        let mut metrics = Self::default();

        for observation in observations {
            match &observation.process.status {
                ProcessStatus::TimedOut => metrics.timed_out += 1,
                ProcessStatus::SpawnFailed { .. } => metrics.failed_to_spawn += 1,
                ProcessStatus::Exited { .. } => {}
            }

            let side_effects = &observation.process.side_effects;
            metrics.side_effect_files_created += side_effects.created;
            metrics.side_effect_files_modified += side_effects.modified;
            metrics.side_effect_files_deleted += side_effects.deleted;
            metrics.side_effect_files_total += side_effects.total;
            if side_effects.total > 0 {
                metrics.side_effect_probe_count += 1;
            }
            metrics.side_effect_scan_truncated |= side_effects.truncated;
            metrics.credential_like_side_effects += side_effects
                .changes
                .iter()
                .filter(|change| path_classification::credential_like_path(&change.path))
                .count();

            let diagnostic = diagnostic::analyze_process(
                &observation.process.status,
                observation.process.stdout.text.as_deref(),
                observation.process.stderr.text.as_deref(),
            );
            let precondition = diagnostic.precondition;
            if let Some(precondition) = precondition {
                metrics.precondition_blocked += 1;
                match precondition {
                    PreconditionKind::AuthRequired => metrics.auth_required += 1,
                    PreconditionKind::LocalContextRequired => metrics.local_context_required += 1,
                    PreconditionKind::FixtureRequired => metrics.fixture_required += 1,
                    PreconditionKind::NetworkUnavailable
                    | PreconditionKind::RuntimeDependencyUnavailable => {}
                }
                if diagnostic.recovery.quality == RecoveryQuality::Actionable {
                    metrics.actionable_precondition += 1;
                }
            }

            if matches!(
                observation.intent,
                ProbeIntent::InvalidCommand | ProbeIntent::InvalidChild | ProbeIntent::InvalidFlag
            ) && precondition.is_none()
            {
                metrics.invalid_probe_count += 1;
                if exited_nonzero(&observation.process.status) {
                    metrics.invalid_probe_rejections += 1;
                }
                if diagnostic.recovery.quality == RecoveryQuality::Actionable {
                    metrics.invalid_probe_actionable += 1;
                }
            }
        }

        metrics
    }
}

pub(super) fn process_text(process: &ProcessCompleted) -> Option<&str> {
    process
        .stdout
        .text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .or_else(|| {
            process
                .stderr
                .text
                .as_deref()
                .filter(|text| !text.trim().is_empty())
        })
}

fn grammar_gaps_for(command: &CommandClaim) -> usize {
    let mut gaps = 2_usize;
    if command.invalid_flag_rejected() {
        gaps = gaps.saturating_sub(1);
    }
    if command.usage_observed()
        || !command.has_child_candidates()
        || command.invalid_child_rejected()
    {
        gaps = gaps.saturating_sub(1);
    }
    gaps
}

fn flag_grammar_known(flag: &FlagClaim) -> bool {
    matches!(flag.value_kind(), FlagValueKind::Boolean) || flag.value_name().is_some()
}

fn exited_nonzero(status: &ProcessStatus) -> bool {
    matches!(status, ProcessStatus::Exited { code: Some(code) } if *code != 0)
}

fn exited_zero(status: &ProcessStatus) -> bool {
    matches!(status, ProcessStatus::Exited { code: Some(0) })
}
