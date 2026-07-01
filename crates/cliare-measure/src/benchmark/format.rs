use super::corpus::ScoreBand;
use super::report_model::BenchmarkTargetReport;

pub(super) fn optional_score(score: Option<f64>) -> String {
    score.map_or_else(|| "n/a".to_owned(), |score| format!("{score:.0}"))
}

pub(super) fn optional_percent(value: Option<f64>) -> String {
    value.map_or_else(
        || "n/a".to_owned(),
        |value| format!("{:.1}%", value * 100.0),
    )
}

pub(super) fn optional_duration(duration_ms: Option<u128>) -> String {
    duration_ms.map_or_else(|| "n/a".to_owned(), |duration| format!("{duration} ms"))
}

pub(super) fn optional_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |value| value.to_string())
}

pub(super) fn optional_depth(observed: Option<usize>, max: Option<usize>) -> String {
    match (observed, max) {
        (Some(observed), Some(max)) => format!("{observed}/{max}"),
        _ => "n/a".to_owned(),
    }
}

pub(super) fn score_range(min: Option<f64>, max: Option<f64>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => format!("{min:.0}..={max:.0}"),
        _ => "n/a".to_owned(),
    }
}

pub(super) fn budget_label(target: &BenchmarkTargetReport) -> String {
    match (
        target.budget_exhausted,
        target.traversal_stop_reason.as_deref(),
    ) {
        (Some(true), Some(reason)) => format!("exhausted:{reason}"),
        (Some(false), Some(reason)) => reason.to_owned(),
        (Some(value), None) => value.to_string(),
        _ => "n/a".to_owned(),
    }
}

pub(super) fn output_label(target: &BenchmarkTargetReport) -> String {
    match (
        target.machine_readable_output_contracts,
        target.output_contracts_discovered,
        target.output_mode_parse_successes,
    ) {
        (Some(machine), Some(discovered), Some(parse_successes)) => {
            format!("{machine}/{discovered}; parse {parse_successes}")
        }
        _ => "n/a".to_owned(),
    }
}

pub(super) fn precondition_label(target: &BenchmarkTargetReport) -> String {
    match (
        target.commands_precondition_blocked,
        target.precondition_blocked_probes,
        target.auth_required_probes,
        target.local_context_required_probes,
        target.fixture_required_probes,
    ) {
        (Some(commands), Some(probes), Some(auth), Some(local), Some(fixture)) => {
            format!("{commands}/{probes}/{auth}/{local}/{fixture}")
        }
        _ => "n/a".to_owned(),
    }
}

pub(super) fn expected_score_label(score: Option<&ScoreBand>) -> String {
    score.map_or_else(
        || "n/a".to_owned(),
        |score| format!("{:.0}..={:.0}", score.min, score.max),
    )
}

pub(super) fn escape_markdown(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}
