use super::{
    Scorecard, dimension_label, dimension_status_label, escape_table_cell, score_label,
    score_status_label, severity_label, traversal_stop_reason_label,
};

pub(super) fn render(scorecard: &Scorecard) -> String {
    let mut report = String::new();

    report.push_str("# CLIARE Report\n\n");
    report.push_str("This report is generated from runtime evidence. Score v0 is experimental and partial: unmeasured dimensions are shown but excluded from the partial total.\n\n");
    report.push_str("## Summary\n\n");
    report.push_str(&format!(
        "- Target: `{}`\n",
        scorecard.target.requested.display()
    ));
    report.push_str(&format!(
        "- Resolved binary: `{}`\n",
        scorecard.target.resolved.display()
    ));
    report.push_str(&format!(
        "- Binary SHA-256: `{}`\n",
        scorecard.target.binary_sha256
    ));
    report.push_str(&format!(
        "- Score: `{:.0}` / 100 (`{}`)\n",
        scorecard.score.total,
        score_status_label(&scorecard.score.status)
    ));
    report.push_str(&format!(
        "- Measured weight: `{:.1}` of `{:.1}`\n",
        scorecard.score.measured_weight, scorecard.score.max_weight
    ));
    report.push_str(&format!("- Model: `{}`\n\n", scorecard.model.name));

    report.push_str("## Runtime Context\n\n");
    report.push_str(&format!(
        "- Profile: `{}`\n",
        scorecard.runtime_context.profile.label()
    ));
    report.push_str(&format!("- Name: `{}`\n", scorecard.runtime_context.name));
    report.push_str(&format!(
        "- Authentication: `{}`\n",
        scorecard.runtime_context.auth_state.label()
    ));
    report.push_str(&format!(
        "- Local context: `{}`\n",
        scorecard.runtime_context.local_context_state.label()
    ));
    report.push_str(&format!(
        "- Fixture data: `{}`\n",
        scorecard.runtime_context.fixture_state.label()
    ));
    report.push_str(&format!(
        "- Network: `{}`\n",
        scorecard.runtime_context.network_state.label()
    ));
    report.push_str(&format!(
        "- Runtime dependencies: `{}`\n",
        scorecard.runtime_context.runtime_dependency_state.label()
    ));
    report.push_str(&format!(
        "- CWD policy: `{}`\n",
        scorecard.runtime_context.cwd_policy.label()
    ));
    if let Some(workdir) = &scorecard.runtime_context.workdir {
        report.push_str(&format!("- Context workdir: `{}`\n", workdir.display()));
    }
    report.push('\n');

    report.push_str("## Runtime Isolation\n\n");
    report.push_str(&format!(
        "- Sandbox profile: `{}`\n",
        scorecard.coverage.sandbox_profile
    ));
    report.push_str(&format!(
        "- Environment policy: `{}`\n",
        scorecard.coverage.sandbox_env_policy
    ));
    report.push_str(&format!(
        "- Sandbox root: `{}`\n",
        scorecard.coverage.sandbox_root.display()
    ));
    report.push_str(&format!(
        "- Sandbox home: `{}`\n",
        scorecard.coverage.sandbox_home.display()
    ));
    report.push_str(&format!(
        "- Sandbox workdir: `{}`\n\n",
        scorecard.coverage.sandbox_workdir.display()
    ));

    report.push_str("## Subscores\n\n");
    report.push_str("| Dimension | Score | Weight | Status | Rationale |\n");
    report.push_str("| --- | ---: | ---: | --- | --- |\n");
    for (dimension, subscore) in &scorecard.subscores {
        report.push_str(&format!(
            "| {} | {} | {:.2} | {} | {} |\n",
            dimension_label(*dimension),
            score_label(subscore.score),
            subscore.weight,
            dimension_status_label(&subscore.status),
            escape_table_cell(&subscore.rationale)
        ));
    }
    report.push('\n');

    report.push_str("## Coverage\n\n");
    report.push_str(&format!(
        "- Commands discovered: `{}`\n",
        scorecard.coverage.commands_discovered
    ));
    report.push_str(&format!(
        "- Commands runtime-confirmed: `{}`\n",
        scorecard.coverage.commands_runtime_confirmed
    ));
    report.push_str(&format!(
        "- Commands precondition-blocked: `{}`\n",
        scorecard.coverage.commands_precondition_blocked
    ));
    report.push_str(&format!(
        "- Command confirmation rate: `{:.1}%`\n",
        scorecard.coverage.command_confirmation_rate * 100.0
    ));
    report.push_str(&format!(
        "- Help-text probes: `{}`\n",
        scorecard.coverage.help_text_probes
    ));
    report.push_str(&format!(
        "- Help-text probes with extracted shape: `{}`\n",
        scorecard.coverage.help_text_probes_with_shape
    ));
    report.push_str(&format!(
        "- Help-text probes without extracted shape: `{}`\n",
        scorecard.coverage.help_text_probes_without_shape
    ));
    report.push_str(&format!(
        "- Help-text probes not recognized as help-like: `{}`\n",
        scorecard.coverage.help_text_probes_not_recognized
    ));
    report.push_str(&format!(
        "- Parser extraction rate: `{:.1}%`\n",
        scorecard.coverage.parser_extraction_rate * 100.0
    ));
    report.push_str(&format!(
        "- Flags discovered: `{}`\n",
        scorecard.coverage.flags_discovered
    ));
    report.push_str(&format!(
        "- Output contracts discovered: `{}`\n",
        scorecard.coverage.output_contracts_discovered
    ));
    report.push_str(&format!(
        "- Machine-readable output contracts: `{}`\n",
        scorecard.coverage.machine_readable_output_contracts
    ));
    report.push_str(&format!(
        "- Output-mode probes completed: `{}`\n",
        scorecard.coverage.output_mode_probes_completed
    ));
    report.push_str(&format!(
        "- Output-mode parse successes: `{}`\n",
        scorecard.coverage.output_mode_parse_successes
    ));
    report.push_str(&format!(
        "- Output-mode precondition-blocked: `{}`\n",
        scorecard.coverage.output_mode_precondition_blocked
    ));
    report.push_str(&format!(
        "- Side-effect file changes: `{}`\n",
        scorecard.coverage.side_effect_files_total
    ));
    report.push_str(&format!(
        "- Side-effect probes: `{}`\n",
        scorecard.coverage.side_effect_probe_count
    ));
    report.push_str(&format!(
        "- Side-effect files created: `{}`\n",
        scorecard.coverage.side_effect_files_created
    ));
    report.push_str(&format!(
        "- Side-effect files modified: `{}`\n",
        scorecard.coverage.side_effect_files_modified
    ));
    report.push_str(&format!(
        "- Side-effect files deleted: `{}`\n",
        scorecard.coverage.side_effect_files_deleted
    ));
    report.push_str(&format!(
        "- Credential-like side-effect paths: `{}`\n",
        scorecard.coverage.credential_like_side_effects
    ));
    report.push_str(&format!(
        "- Average command confidence: `{:.3}`\n",
        scorecard.coverage.avg_command_confidence
    ));
    report.push_str(&format!(
        "- Average flag confidence: `{:.3}`\n",
        scorecard.coverage.avg_flag_confidence
    ));
    report.push_str(&format!(
        "- Observed max command depth: `{}`\n",
        scorecard.coverage.observed_max_depth
    ));
    report.push_str(&format!(
        "- Traversal profile: `{}`\n",
        scorecard.coverage.traversal_profile
    ));
    report.push_str(&format!(
        "- Depth budget: `{}`\n",
        scorecard.coverage.max_depth
    ));
    report.push_str(&format!(
        "- Probe budget: `{}`\n",
        scorecard.coverage.max_probes
    ));
    report.push_str(&format!(
        "- Minimum expected probe value: `{}`\n",
        scorecard.coverage.min_expected_value
    ));
    report.push_str(&format!(
        "- Concurrency limit: `{}`\n",
        scorecard.coverage.concurrency_limit
    ));
    report.push_str(&format!(
        "- Scheduler rounds: `{}`\n",
        scorecard.coverage.traversal_rounds
    ));
    report.push_str(&format!(
        "- Probes scheduled: `{}`\n",
        scorecard.coverage.probes_scheduled
    ));
    report.push_str(&format!(
        "- Probes completed: `{}`\n",
        scorecard.coverage.probes_completed
    ));
    report.push_str(&format!(
        "- Probes cancelled: `{}`\n",
        scorecard.coverage.probes_cancelled
    ));
    report.push_str(&format!(
        "- Probe timeouts: `{}`\n",
        scorecard.coverage.probes_timed_out
    ));
    report.push_str(&format!(
        "- Probe spawn failures: `{}`\n\n",
        scorecard.coverage.probes_failed_to_spawn
    ));
    report.push_str(&format!(
        "- Precondition-blocked probes: `{}`\n",
        scorecard.coverage.precondition_blocked_probes
    ));
    report.push_str(&format!(
        "- Auth-required probes: `{}`\n",
        scorecard.coverage.auth_required_probes
    ));
    report.push_str(&format!(
        "- Local-context-required probes: `{}`\n",
        scorecard.coverage.local_context_required_probes
    ));
    report.push_str(&format!(
        "- Fixture-required probes: `{}`\n",
        scorecard.coverage.fixture_required_probes
    ));
    report.push_str(&format!(
        "- Actionable precondition diagnostics: `{}`\n",
        scorecard.coverage.actionable_precondition_probes
    ));
    report.push_str(&format!(
        "- Precondition recovery rate: `{:.1}%`\n\n",
        scorecard.coverage.precondition_recovery_rate * 100.0
    ));
    report.push_str(&format!(
        "- Frontier remaining: `{}`\n",
        scorecard.coverage.frontier_remaining
    ));
    report.push_str(&format!(
        "- Highest pending expected value: `{}`\n",
        scorecard
            .coverage
            .highest_pending_expected_value
            .map_or_else(|| "none".to_owned(), |value| value.to_string())
    ));
    report.push_str(&format!(
        "- Candidates skipped by depth: `{}`\n",
        scorecard.coverage.candidates_skipped_by_depth
    ));
    report.push_str(&format!(
        "- Candidates skipped by convergence: `{}`\n",
        scorecard.coverage.candidates_skipped_by_convergence
    ));
    report.push_str(&format!(
        "- Probes skipped by budget: `{}`\n",
        scorecard.coverage.probes_skipped_by_budget
    ));
    report.push_str(&format!(
        "- Budget exhausted: `{}`\n\n",
        scorecard.coverage.budget_exhausted
    ));
    report.push_str(&format!(
        "- Traversal stop reason: `{}`\n",
        traversal_stop_reason_label(scorecard.coverage.traversal_stop_reason)
    ));
    report.push_str(&format!(
        "- Traversal complete: `{}`\n\n",
        scorecard.coverage.traversal_complete
    ));

    report.push_str("## Findings\n\n");
    if scorecard.findings.is_empty() {
        report.push_str("No findings for measured dimensions.\n");
    } else {
        for finding in &scorecard.findings {
            report.push_str(&format!(
                "### {}: {}\n\n",
                severity_label(&finding.severity),
                finding.title
            ));
            report.push_str(&format!("- ID: `{}`\n", finding.id));
            report.push_str(&format!(
                "- Dimension: `{}`\n",
                dimension_label(finding.dimension)
            ));
            report.push_str(&format!("- Detail: {}\n", finding.detail));
            report.push_str(&format!("- Recommendation: {}\n\n", finding.recommendation));
        }
    }

    report
}
