use std::collections::BTreeSet;
use std::path::Path;

use super::model::{
    EvidenceCompleteness, MetricReport, ProvenanceReport, ShapeArtifact, ShapeOutputContract,
    ShapeQualityMetrics, ShapeQualityOverall, ShapeQualityReport, ShapeTruth, TruthCommand,
};

pub(super) fn evaluate_shape_quality(
    schema_version: &'static str,
    shape_path: &Path,
    truth_path: &Path,
    shape: &ShapeArtifact,
    truth: &ShapeTruth,
) -> ShapeQualityReport {
    let metrics = ShapeQualityMetrics {
        commands: metric(command_truth(truth), command_observed(shape)),
        aliases: metric(alias_truth(truth), alias_observed(shape)),
        flags: metric(flag_truth(truth), flag_observed(shape)),
        flag_grammar: metric(flag_grammar_truth(truth), flag_grammar_observed(shape)),
        positionals: metric(positional_truth(truth), positional_observed(shape)),
        output_contracts: metric(output_truth(truth), output_observed(shape)),
        preconditions: metric(precondition_truth(truth), precondition_observed(shape)),
    };

    ShapeQualityReport {
        schema_version,
        shape_path: shape_path.display().to_string(),
        truth_path: truth_path.display().to_string(),
        shape_schema_version: shape.schema_version.clone(),
        truth_schema_version: truth.schema_version.clone(),
        target_id: truth.target_id.clone(),
        overall: overall(&metrics),
        metrics,
        provenance: provenance(shape),
    }
}

fn overall(metrics: &ShapeQualityMetrics) -> ShapeQualityOverall {
    let scored: Vec<f64> = [
        &metrics.commands,
        &metrics.aliases,
        &metrics.flags,
        &metrics.flag_grammar,
        &metrics.positionals,
        &metrics.output_contracts,
        &metrics.preconditions,
    ]
    .into_iter()
    .filter_map(|metric| metric.f1)
    .collect();
    let mean_f1 =
        (!scored.is_empty()).then(|| round4(scored.iter().sum::<f64>() / scored.len() as f64));

    ShapeQualityOverall {
        score: mean_f1.map(|value| round1(value * 100.0)),
        mean_f1,
        metrics_scored: scored.len(),
    }
}

fn metric(expected: BTreeSet<String>, observed: BTreeSet<String>) -> MetricReport {
    let matched = expected.intersection(&observed).count();
    let precision = ratio(matched, observed.len());
    let recall = ratio(matched, expected.len());
    let f1 = f1(precision, recall, expected.len(), observed.len());
    let missing = expected.difference(&observed).cloned().collect();
    let unexpected = observed.difference(&expected).cloned().collect();

    MetricReport {
        expected: expected.len(),
        observed: observed.len(),
        matched,
        precision,
        recall,
        f1,
        missing,
        unexpected,
    }
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then(|| round4(numerator as f64 / denominator as f64))
}

fn f1(
    precision: Option<f64>,
    recall: Option<f64>,
    expected: usize,
    observed: usize,
) -> Option<f64> {
    match (precision, recall) {
        (Some(precision), Some(recall)) if precision + recall > 0.0 => {
            Some(round4((2.0 * precision * recall) / (precision + recall)))
        }
        (Some(_), Some(_)) => Some(0.0),
        _ if expected > 0 || observed > 0 => Some(0.0),
        _ => None,
    }
}

fn command_truth(truth: &ShapeTruth) -> BTreeSet<String> {
    truth
        .commands
        .iter()
        .map(|command| path_key(&command.path))
        .collect()
}

fn command_observed(shape: &ShapeArtifact) -> BTreeSet<String> {
    shape
        .commands
        .iter()
        .map(|command| path_key(&command.path))
        .collect()
}

fn alias_truth(truth: &ShapeTruth) -> BTreeSet<String> {
    truth
        .commands
        .iter()
        .flat_map(|command| {
            command
                .aliases
                .iter()
                .map(|alias| command_key(&command.path, alias))
        })
        .collect()
}

fn alias_observed(shape: &ShapeArtifact) -> BTreeSet<String> {
    shape
        .commands
        .iter()
        .flat_map(|command| {
            command
                .aliases
                .iter()
                .map(|alias| command_key(&command.path, alias))
        })
        .collect()
}

fn flag_truth(truth: &ShapeTruth) -> BTreeSet<String> {
    truth
        .commands
        .iter()
        .flat_map(|command| {
            command
                .flags
                .iter()
                .map(|flag| command_key(&command.path, &flag.name))
        })
        .collect()
}

fn flag_observed(shape: &ShapeArtifact) -> BTreeSet<String> {
    shape
        .flags
        .iter()
        .map(|flag| command_key(&flag.command_path, &flag.name))
        .collect()
}

fn flag_grammar_truth(truth: &ShapeTruth) -> BTreeSet<String> {
    truth
        .commands
        .iter()
        .flat_map(|command| {
            command.flags.iter().map(|flag| {
                components_key([
                    path_key(&command.path),
                    flag.name.clone(),
                    flag.short.clone().unwrap_or_default(),
                    flag.value_kind.clone(),
                    flag.value_name.clone().unwrap_or_default(),
                    flag.required.to_string(),
                    flag.repeatable.to_string(),
                ])
            })
        })
        .collect()
}

fn flag_grammar_observed(shape: &ShapeArtifact) -> BTreeSet<String> {
    shape
        .flags
        .iter()
        .map(|flag| {
            components_key([
                path_key(&flag.command_path),
                flag.name.clone(),
                flag.short.clone().unwrap_or_default(),
                flag.value_kind.clone(),
                flag.value_name.clone().unwrap_or_default(),
                flag.required.to_string(),
                flag.repeatable.to_string(),
            ])
        })
        .collect()
}

fn positional_truth(truth: &ShapeTruth) -> BTreeSet<String> {
    truth
        .commands
        .iter()
        .flat_map(|command| {
            command
                .positionals
                .iter()
                .enumerate()
                .map(|(index, positional)| {
                    components_key([
                        path_key(&command.path),
                        index.to_string(),
                        positional.name.clone(),
                        positional.required.to_string(),
                        positional.variadic.to_string(),
                    ])
                })
        })
        .collect()
}

fn positional_observed(shape: &ShapeArtifact) -> BTreeSet<String> {
    shape
        .commands
        .iter()
        .flat_map(|command| {
            command
                .positionals
                .iter()
                .enumerate()
                .map(|(index, positional)| {
                    components_key([
                        path_key(&command.path),
                        index.to_string(),
                        positional.name.clone(),
                        positional.required.to_string(),
                        positional.variadic.to_string(),
                    ])
                })
        })
        .collect()
}

fn output_truth(truth: &ShapeTruth) -> BTreeSet<String> {
    truth
        .commands
        .iter()
        .flat_map(|command| {
            command.output_contracts.iter().map(|contract| {
                output_key(
                    &command.path,
                    &contract.mode,
                    &contract.flag_name,
                    &contract.argv_fragment,
                    contract.parseable,
                )
            })
        })
        .collect()
}

fn output_observed(shape: &ShapeArtifact) -> BTreeSet<String> {
    shape
        .output_contracts
        .iter()
        .map(|contract| {
            output_key(
                &contract.command_path,
                &contract.mode,
                &contract.flag_name,
                &contract.argv_fragment,
                contract.parse_success,
            )
        })
        .collect()
}

fn output_key(
    command_path: &[String],
    mode: &str,
    flag_name: &str,
    argv_fragment: &[String],
    parseable: bool,
) -> String {
    components_key([
        path_key(command_path),
        mode.to_owned(),
        flag_name.to_owned(),
        argv_fragment.join(" "),
        parseable.to_string(),
    ])
}

fn precondition_truth(truth: &ShapeTruth) -> BTreeSet<String> {
    truth
        .commands
        .iter()
        .flat_map(preconditions_for_command)
        .collect()
}

fn preconditions_for_command(command: &TruthCommand) -> impl Iterator<Item = String> + '_ {
    command
        .preconditions
        .iter()
        .map(|precondition| command_key(&command.path, precondition))
}

fn precondition_observed(shape: &ShapeArtifact) -> BTreeSet<String> {
    let command_preconditions = shape.commands.iter().flat_map(|command| {
        command
            .preconditions
            .iter()
            .map(|precondition| command_key(&command.path, precondition))
    });
    let output_preconditions = shape
        .output_contracts
        .iter()
        .flat_map(output_contract_preconditions);

    command_preconditions.chain(output_preconditions).collect()
}

fn output_contract_preconditions(
    contract: &ShapeOutputContract,
) -> impl Iterator<Item = String> + '_ {
    contract
        .preconditions
        .iter()
        .map(|precondition| command_key(&contract.command_path, precondition))
}

fn provenance(shape: &ShapeArtifact) -> ProvenanceReport {
    ProvenanceReport {
        target_present: !shape.target.is_null(),
        model_present: !shape.model.is_null(),
        command_evidence: evidence_completeness(
            shape
                .commands
                .iter()
                .map(|command| command.evidence.is_empty()),
        ),
        flag_evidence: evidence_completeness(
            shape.flags.iter().map(|flag| flag.evidence.is_empty()),
        ),
        output_contract_evidence: evidence_completeness(
            shape
                .output_contracts
                .iter()
                .map(|contract| contract.evidence.is_empty()),
        ),
    }
}

fn evidence_completeness(empty_values: impl Iterator<Item = bool>) -> EvidenceCompleteness {
    let mut total = 0;
    let mut with_evidence = 0;
    for empty in empty_values {
        total += 1;
        if !empty {
            with_evidence += 1;
        }
    }

    EvidenceCompleteness {
        total,
        with_evidence,
        rate: ratio(with_evidence, total),
    }
}

fn command_key(path: &[String], suffix: &str) -> String {
    components_key([path_key(path), suffix.to_owned()])
}

fn path_key(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(" ")
    }
}

fn components_key<const N: usize>(components: [String; N]) -> String {
    components.join(" | ")
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
