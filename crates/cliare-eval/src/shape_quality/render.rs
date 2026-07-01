use super::model::{EvidenceCompleteness, MetricReport, ShapeQualityReport};

pub(super) fn render_report(report: &ShapeQualityReport) -> String {
    let mut out = Vec::new();
    out.push("# CLIARE Shape Quality".to_owned());
    out.push(String::new());
    out.push(format!("- Shape: `{}`", report.shape_path));
    out.push(format!("- Truth: `{}`", report.truth_path));
    out.push(format!("- Shape schema: `{}`", report.shape_schema_version));
    out.push(format!("- Truth schema: `{}`", report.truth_schema_version));
    if let Some(target_id) = &report.target_id {
        out.push(format!("- Target id: `{target_id}`"));
    }
    out.push(format!(
        "- Overall score: `{}`",
        optional_score(report.overall.score)
    ));
    out.push(String::new());
    out.push("## Accuracy".to_owned());
    out.push(String::new());
    out.push("| Metric | Expected | Observed | Matched | Precision | Recall | F1 |".to_owned());
    out.push("|---|---:|---:|---:|---:|---:|---:|".to_owned());
    push_metric(&mut out, "Commands", &report.metrics.commands);
    push_metric(&mut out, "Aliases", &report.metrics.aliases);
    push_metric(&mut out, "Flags", &report.metrics.flags);
    push_metric(&mut out, "Flag grammar", &report.metrics.flag_grammar);
    push_metric(&mut out, "Positionals", &report.metrics.positionals);
    push_metric(
        &mut out,
        "Output contracts",
        &report.metrics.output_contracts,
    );
    push_metric(&mut out, "Preconditions", &report.metrics.preconditions);
    out.push(String::new());
    out.push("## Provenance".to_owned());
    out.push(String::new());
    out.push(format!(
        "- Target present: `{}`",
        report.provenance.target_present
    ));
    out.push(format!(
        "- Model present: `{}`",
        report.provenance.model_present
    ));
    out.push(format!(
        "- Command evidence: {}",
        evidence(&report.provenance.command_evidence)
    ));
    out.push(format!(
        "- Flag evidence: {}",
        evidence(&report.provenance.flag_evidence)
    ));
    out.push(format!(
        "- Output contract evidence: {}",
        evidence(&report.provenance.output_contract_evidence)
    ));
    out.push(String::new());

    format!("{}\n", out.join("\n"))
}

fn push_metric(out: &mut Vec<String>, label: &str, metric: &MetricReport) {
    out.push(format!(
        "| {label} | {} | {} | {} | {} | {} | {} |",
        metric.expected,
        metric.observed,
        metric.matched,
        optional_ratio(metric.precision),
        optional_ratio(metric.recall),
        optional_ratio(metric.f1)
    ));
}

fn optional_score(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn optional_ratio(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn evidence(value: &EvidenceCompleteness) -> String {
    format!(
        "`{}/{}` (`{}`)",
        value.with_evidence,
        value.total,
        optional_ratio(value.rate)
    )
}
