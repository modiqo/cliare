use cliare_inference::score_model::Normalization;
use cliare_runtime::sandbox::EnvPolicy;

use super::Dimension;
use super::model::{DimensionStatus, ScoreStatus, Severity, TraversalStopReason};

pub(super) fn score_label(score: Option<f64>) -> String {
    score.map_or_else(|| "not measured".to_owned(), |score| format!("{score:.0}"))
}

pub(super) fn score_status_label(status: &ScoreStatus) -> &'static str {
    match status {
        ScoreStatus::ExperimentalPartial => "experimental partial",
    }
}

pub(super) fn normalization_label(normalization: Normalization) -> &'static str {
    match normalization {
        Normalization::DeclaredWeight => "declared_weight",
    }
}

pub(super) fn dimension_label(dimension: Dimension) -> &'static str {
    match dimension {
        Dimension::Discovery => "discovery",
        Dimension::Grammar => "grammar",
        Dimension::Execution => "execution",
        Dimension::Output => "output",
        Dimension::Safety => "safety",
        Dimension::Recovery => "recovery",
    }
}

pub(super) fn dimension_status_label(status: &DimensionStatus) -> &'static str {
    match status {
        DimensionStatus::Measured => "measured",
        DimensionStatus::NotMeasured => "not measured",
    }
}

pub(super) fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Low => "Low",
        Severity::Medium => "Medium",
        Severity::High => "High",
    }
}

pub(super) fn traversal_stop_reason_label(reason: TraversalStopReason) -> &'static str {
    match reason {
        TraversalStopReason::FrontierExhausted => "frontier_exhausted",
        TraversalStopReason::Converged => "converged",
        TraversalStopReason::DepthBudgetExhausted => "depth_budget_exhausted",
        TraversalStopReason::ProbeBudgetExhausted => "probe_budget_exhausted",
    }
}

pub(super) fn env_policy_label(policy: EnvPolicy) -> &'static str {
    match policy {
        EnvPolicy::ClearedWithAllowlist => "cleared_with_allowlist",
        EnvPolicy::Inherited => "inherited",
    }
}

pub(super) fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|")
}
