use std::collections::BTreeMap;

use cliare_inference::score_model::ScoreModelSpec;

use super::Dimension;
use super::metrics::Metrics;
use super::model::{DimensionScore, DimensionStatus, ScoreStatus, ScoreSummary};
use super::util::{ratio, round_score, round_weight};

pub(super) fn subscores(
    metrics: &Metrics,
    model: &ScoreModelSpec,
) -> BTreeMap<Dimension, DimensionScore> {
    let mut subscores = BTreeMap::new();

    subscores.insert(
        Dimension::Discovery,
        DimensionScore {
            score: Some(round_score(
                model.scoring.discovery.recognition_weight * metrics.command_recognition_rate()
                    + model.scoring.discovery.confidence_weight
                        * metrics.coverage.avg_command_confidence,
            )),
            weight: model.weight(Dimension::Discovery),
            status: DimensionStatus::Measured,
            rationale: "confirmed command coverage plus average command confidence".to_owned(),
        },
    );
    subscores.insert(
        Dimension::Grammar,
        DimensionScore {
            score: Some(round_score(grammar_score(metrics, model))),
            weight: model.weight(Dimension::Grammar),
            status: DimensionStatus::Measured,
            rationale: "flag discovery, flag confidence, and confirmed-command grammar gaps"
                .to_owned(),
        },
    );
    subscores.insert(
        Dimension::Execution,
        DimensionScore {
            score: Some(round_score(execution_score(metrics))),
            weight: model.weight(Dimension::Execution),
            status: DimensionStatus::Measured,
            rationale: "probe completion without timeouts or spawn failures".to_owned(),
        },
    );
    subscores.insert(
        Dimension::Recovery,
        DimensionScore {
            score: Some(round_score(recovery_score(metrics, model))),
            weight: model.weight(Dimension::Recovery),
            status: DimensionStatus::Measured,
            rationale: "invalid-command, invalid-child, and invalid-flag probes reject cleanly"
                .to_owned(),
        },
    );
    subscores.insert(
        Dimension::Output,
        DimensionScore {
            score: Some(round_score(output_score(metrics, model))),
            weight: model.weight(Dimension::Output),
            status: DimensionStatus::Measured,
            rationale: "advertised machine-readable output modes and safe parse probes".to_owned(),
        },
    );
    subscores.insert(Dimension::Safety, safety_dimension_score(metrics, model));

    subscores
}

pub(super) fn safety_dimension_score(metrics: &Metrics, model: &ScoreModelSpec) -> DimensionScore {
    let weight = model.weight(Dimension::Safety);
    if !metrics.side_effect_observation_supported() {
        return DimensionScore {
            score: None,
            weight,
            status: DimensionStatus::NotMeasured,
            rationale: "host execution does not register filesystem snapshot regions".to_owned(),
        };
    }
    if metrics.coverage.side_effect_scan_truncated {
        return DimensionScore {
            score: None,
            weight,
            status: DimensionStatus::NotMeasured,
            rationale: "filesystem side-effect snapshot exceeded scanner budget".to_owned(),
        };
    }

    DimensionScore {
        score: Some(round_score(safety_score(metrics, model))),
        weight,
        status: DimensionStatus::Measured,
        rationale: "persistent sandbox filesystem side effects from safe probes".to_owned(),
    }
}

pub(super) fn total_score(
    subscores: &BTreeMap<Dimension, DimensionScore>,
    model: &ScoreModelSpec,
) -> ScoreSummary {
    let mut weighted = 0.0;
    let mut measured_weight = 0.0;
    let mut max_weight = 0.0;

    for subscore in subscores.values() {
        max_weight += subscore.weight;
        if let Some(score) = subscore.score {
            weighted += score * subscore.weight;
            measured_weight += subscore.weight;
        }
    }

    let total = if max_weight > 0.0 {
        weighted / max_weight
    } else {
        0.0
    };

    ScoreSummary {
        total: round_score(total),
        measured_weight: round_weight(measured_weight),
        max_weight: round_weight(max_weight),
        model: model.id.clone(),
        status: ScoreStatus::ExperimentalPartial,
    }
}

pub(super) fn grammar_score(metrics: &Metrics, model: &ScoreModelSpec) -> f64 {
    if metrics.coverage.commands_runtime_confirmed == 0 {
        return 0.0;
    }

    let flag_presence = if metrics.coverage.flags_discovered > 0 {
        1.0
    } else {
        0.0
    };
    let grammar_gap_rate = metrics.grammar_gap_rate();

    model.scoring.grammar.flag_presence_score * flag_presence
        + model.scoring.grammar.flag_confidence_score * metrics.coverage.avg_flag_confidence
        + model.scoring.grammar.flag_grammar_score * metrics.flag_grammar_rate()
        + model.scoring.grammar.command_gap_score * (1.0 - grammar_gap_rate)
}

pub(super) fn execution_score(metrics: &Metrics) -> f64 {
    if metrics.coverage.probes_completed == 0 {
        return 0.0;
    }

    let bad = metrics.coverage.probes_timed_out + metrics.coverage.probes_failed_to_spawn;
    100.0 * (1.0 - ratio(bad, metrics.coverage.probes_completed))
}

pub(super) fn recovery_score(metrics: &Metrics, model: &ScoreModelSpec) -> f64 {
    let invalid_recovery = if metrics.invalid_probe_count == 0 {
        None
    } else {
        Some(ratio(
            metrics.invalid_probe_rejections,
            metrics.invalid_probe_count,
        ))
    };
    let precondition_recovery = if metrics.coverage.precondition_blocked_probes == 0 {
        None
    } else {
        Some(metrics.coverage.precondition_recovery_rate)
    };

    100.0
        * match (invalid_recovery, precondition_recovery) {
            (Some(invalid), Some(precondition)) => {
                model.scoring.recovery.invalid_probe_weight * invalid
                    + model.scoring.recovery.precondition_recovery_weight * precondition
            }
            (Some(invalid), None) => invalid,
            (None, Some(precondition)) => precondition,
            (None, None) => 0.0,
        }
}

pub(super) fn output_score(metrics: &Metrics, model: &ScoreModelSpec) -> f64 {
    if metrics.machine_readable_output_contracts == 0 {
        return 0.0;
    }

    let non_blocked_probe_count = metrics
        .output_mode_probe_count
        .saturating_sub(metrics.output_mode_precondition_blocked)
        .saturating_sub(metrics.output_mode_help_text_probes)
        .saturating_sub(metrics.output_mode_global_scope_failures);
    let denominator = metrics
        .output_mode_scored_contracts
        .max(non_blocked_probe_count);
    model.scoring.output.advertised_score
        + model.scoring.output.parse_score * ratio(metrics.output_mode_parse_successes, denominator)
}

pub(super) fn safety_score(metrics: &Metrics, model: &ScoreModelSpec) -> f64 {
    if metrics.coverage.probes_completed == 0 {
        return 0.0;
    }

    let changed_probe_penalty = model.scoring.safety.changed_probe_penalty
        * ratio(
            metrics.side_effect_probe_count,
            metrics.coverage.probes_completed,
        );
    let file_penalty = (metrics.side_effect_files_total as f64
        * model.scoring.safety.file_penalty_per_file)
        .min(model.scoring.safety.file_penalty_cap);
    let credential_penalty = (metrics.credential_like_side_effects as f64
        * model.scoring.safety.credential_penalty_per_path)
        .min(model.scoring.safety.credential_penalty_cap);

    (100.0 - changed_probe_penalty - file_penalty - credential_penalty).max(0.0)
}
