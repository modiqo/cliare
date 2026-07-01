use cliare_inference::score_model::ScoreModelSpec;

use super::Dimension;
use super::metrics::Metrics;
use super::model::{Finding, Severity};
use super::util::ratio;

pub(super) fn findings(metrics: &Metrics, model: &ScoreModelSpec) -> Vec<Finding> {
    let mut findings = Vec::new();

    if metrics.coverage.command_confirmation_rate < model.thresholds.low_runtime_confirmation
        && metrics.coverage.commands_discovered > 0
    {
        findings.push(Finding {
            id: "finding.discovery.low_runtime_confirmation",
            dimension: Dimension::Discovery,
            severity: Severity::High,
            title: "Most discovered command candidates are not runtime-confirmed",
            detail: format!(
                "{} of {} command candidates were runtime-confirmed; {} were blocked by runtime preconditions.",
                metrics.coverage.commands_runtime_confirmed,
                metrics.coverage.commands_discovered,
                metrics.coverage.commands_precondition_blocked
            ),
            recommendation: "Increase probe budget, improve help consistency, or expose a clearer command catalog.",
        });
    }

    if metrics.extraction.measurement_limited(model) {
        findings.push(Finding {
            id: "finding.discovery.extraction_limited",
            dimension: Dimension::Discovery,
            severity: Severity::Medium,
            title: "Help text was not converted into reliable command shape",
            detail: format!(
                "{} help-text probes produced output, but none yielded structural shape signals. Help-like-but-unparsed: {}; not recognized as help-like: {}.",
                metrics.coverage.help_text_probes,
                metrics.coverage.help_text_probes_without_shape,
                metrics.coverage.help_text_probes_not_recognized
            ),
            recommendation: "Treat discovery and grammar scores as measurement-limited until the help layout is reviewed, a machine-readable catalog is provided, or CLIARE parser support is improved.",
        });
    }

    if metrics.grammar_gap_rate() > model.thresholds.grammar_gap_rate
        && metrics.coverage.commands_runtime_confirmed > 0
    {
        findings.push(Finding {
            id: "finding.grammar.unconfirmed_arity",
            dimension: Dimension::Grammar,
            severity: Severity::Medium,
            title: "Confirmed commands still have unknown grammar details",
            detail: format!(
                "{} grammar gaps remain across {} runtime-confirmed commands.",
                metrics.grammar_gap_count, metrics.coverage.commands_runtime_confirmed
            ),
            recommendation: "Add explicit usage syntax, flag arity, and value-domain diagnostics.",
        });
    }

    if metrics.coverage.probes_timed_out > 0 {
        findings.push(Finding {
            id: "finding.execution.timeouts",
            dimension: Dimension::Execution,
            severity: Severity::High,
            title: "Some probes timed out",
            detail: format!("{} probes timed out.", metrics.coverage.probes_timed_out),
            recommendation: "Ensure help and diagnostic paths are fast and noninteractive under CI=1.",
        });
    }

    if metrics.invalid_probe_count > 0
        && invalid_probe_recovery_score(metrics) < model.thresholds.recovery_score_minimum
    {
        findings.push(Finding {
            id: "finding.recovery.invalid_probe_acceptance",
            dimension: Dimension::Recovery,
            severity: Severity::Medium,
            title: "Invalid probes did not consistently reject",
            detail: format!(
                "{} of {} invalid probes rejected with nonzero exit status.",
                metrics.invalid_probe_rejections, metrics.invalid_probe_count
            ),
            recommendation: "Reject unknown commands and flags with clear diagnostics and nonzero exit codes.",
        });
    }

    if metrics.coverage.machine_readable_output_contracts == 0 {
        findings.push(Finding {
            id: "finding.output.no_machine_readable_mode",
            dimension: Dimension::Output,
            severity: Severity::Medium,
            title: "No machine-readable output mode was discovered",
            detail: "No JSON or YAML output contract was found in runtime help evidence."
                .to_owned(),
            recommendation: "Advertise a stable JSON or YAML output mode in command help.",
        });
    } else if metrics.output_mode_parse_failures() > 0 {
        findings.push(Finding {
            id: "finding.output.unparseable_mode",
            dimension: Dimension::Output,
            severity: Severity::High,
            title: "Some advertised output modes did not parse",
            detail: format!(
                "{} of {} non-blocked output-mode probes parsed successfully.",
                metrics.coverage.output_mode_parse_successes,
                metrics
                    .output_mode_probe_count
                    .saturating_sub(metrics.output_mode_precondition_blocked)
                    .saturating_sub(metrics.output_mode_help_text_probes)
                    .saturating_sub(metrics.output_mode_global_scope_failures)
            ),
            recommendation: "Ensure documented machine-readable modes produce valid output for safe help or diagnostic probes.",
        });
    }

    if metrics.coverage.precondition_blocked_probes > 0 {
        findings.push(Finding {
            id: "finding.precondition.runtime_blocked",
            dimension: Dimension::Discovery,
            severity: Severity::Medium,
            title: "Some probes were blocked by runtime preconditions",
            detail: format!(
                "{} probes were blocked by runtime preconditions across {} command candidates.",
                metrics.coverage.precondition_blocked_probes,
                metrics.coverage.commands_precondition_blocked
            ),
            recommendation: "Document required runtime preconditions separately from command existence, and keep help paths available where practical.",
        });
    }

    if !metrics.side_effect_observation_supported() {
        findings.push(Finding {
            id: "finding.safety.side_effects_unobserved_in_host_mode",
            dimension: Dimension::Safety,
            severity: Severity::Medium,
            title: "Host-mode filesystem side effects were not observed",
            detail: "This run used host execution mode, which intentionally does not register filesystem snapshot regions. Side-effect totals are unmeasured, not proof of clean behavior.".to_owned(),
            recommendation: "Use isolated execution for filesystem side-effect scoring, or treat host-mode safety as a context-specific manual review input.",
        });
    }

    if metrics.coverage.side_effect_scan_truncated {
        findings.push(Finding {
            id: "finding.safety.side_effect_scan_truncated",
            dimension: Dimension::Safety,
            severity: Severity::High,
            title: "Filesystem side-effect scanning was truncated",
            detail: "The sandbox snapshot exceeded a scanner budget. Side-effect totals are partial, so filesystem safety is not fully measured.".to_owned(),
            recommendation: "Reduce discovery-time filesystem writes or raise scanner limits only in a controlled profile with explicit review.",
        });
    }

    if metrics.coverage.side_effect_files_total > 0 {
        findings.push(Finding {
            id: "finding.safety.safe_probe_side_effects",
            dimension: Dimension::Safety,
            severity: Severity::High,
            title: "Safe probes left persistent filesystem side effects",
            detail: format!(
                "{} file changes were observed across {} probes.",
                metrics.coverage.side_effect_files_total,
                metrics.coverage.side_effect_probe_count
            ),
            recommendation: "Keep help, version, and diagnostic paths read-only, or clearly document unavoidable cache/config writes.",
        });
    }

    if metrics.coverage.credential_like_side_effects > 0 {
        findings.push(Finding {
            id: "finding.safety.credential_like_side_effects",
            dimension: Dimension::Safety,
            severity: Severity::High,
            title: "Side-effect paths looked credential-related",
            detail: format!(
                "{} side-effect paths contained credential-like terms.",
                metrics.coverage.credential_like_side_effects
            ),
            recommendation: "Do not create token, secret, credential, or key material during discovery probes.",
        });
    }

    findings
}

fn invalid_probe_recovery_score(metrics: &Metrics) -> f64 {
    100.0
        * ratio(
            metrics.invalid_probe_rejections,
            metrics.invalid_probe_count,
        )
}
