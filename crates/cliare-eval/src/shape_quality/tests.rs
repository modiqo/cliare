use std::path::Path;

use super::metrics::evaluate_shape_quality;
use super::model::{ShapeArtifact, ShapeTruth};

fn shape_artifact() -> ShapeArtifact {
    serde_json::from_value(serde_json::json!({
        "schema_version": "cliare.shape.v1",
        "target": { "requested": "fixture-cli" },
        "model": { "name": "test" },
        "commands": [
            {
                "path": [],
                "aliases": [],
                "positionals": [],
                "preconditions": [],
                "evidence": ["e_1"]
            },
            {
                "path": ["project", "list"],
                "aliases": ["ls"],
                "positionals": [],
                "preconditions": [],
                "evidence": ["e_2"]
            },
            {
                "path": ["project", "delete"],
                "aliases": [],
                "positionals": [{ "name": "id", "required": true, "variadic": false }],
                "preconditions": ["auth_required"],
                "evidence": []
            }
        ],
        "flags": [
            {
                "command_path": ["project", "list"],
                "name": "--format",
                "short": "-f",
                "value_kind": "required",
                "value_name": "FORMAT",
                "required": false,
                "repeatable": false,
                "evidence": ["e_3"]
            },
            {
                "command_path": ["project", "delete"],
                "name": "--force",
                "value_kind": "boolean",
                "required": false,
                "repeatable": false,
                "evidence": []
            }
        ],
        "output_contracts": [
            {
                "command_path": ["project", "list"],
                "mode": "json",
                "flag_name": "--format",
                "argv_fragment": ["--format", "json"],
                "parse_success": true,
                "preconditions": [],
                "evidence": ["e_4"]
            }
        ]
    }))
    .expect("shape fixture parses")
}

fn shape_truth() -> ShapeTruth {
    serde_json::from_value(serde_json::json!({
        "schema_version": "cliare.shape-truth.v1",
        "target_id": "fixture-cli",
        "commands": [
            {
                "path": [],
                "aliases": [],
                "positionals": [],
                "flags": [],
                "output_contracts": [],
                "preconditions": []
            },
            {
                "path": ["project", "list"],
                "aliases": ["ls"],
                "positionals": [],
                "flags": [
                    {
                        "name": "--format",
                        "short": "-f",
                        "value_kind": "required",
                        "value_name": "FORMAT",
                        "required": false,
                        "repeatable": false
                    }
                ],
                "output_contracts": [
                    {
                        "mode": "json",
                        "flag_name": "--format",
                        "argv_fragment": ["--format", "json"],
                        "parseable": true
                    }
                ],
                "preconditions": []
            },
            {
                "path": ["project", "remove"],
                "aliases": [],
                "positionals": [{ "name": "id", "required": true, "variadic": false }],
                "flags": [],
                "output_contracts": [],
                "preconditions": ["auth_required"]
            }
        ]
    }))
    .expect("truth fixture parses")
}

#[test]
fn evaluates_shape_against_truth_sets() {
    let report = evaluate_shape_quality(
        "cliare.shape-quality.v1",
        Path::new("shape.json"),
        Path::new("truth.json"),
        &shape_artifact(),
        &shape_truth(),
    );

    assert_eq!(report.metrics.commands.expected, 3);
    assert_eq!(report.metrics.commands.observed, 3);
    assert_eq!(report.metrics.commands.matched, 2);
    assert_eq!(report.metrics.commands.f1, Some(0.6667));
    assert_eq!(
        report.metrics.commands.missing,
        vec!["project remove".to_owned()]
    );
    assert_eq!(
        report.metrics.commands.unexpected,
        vec!["project delete".to_owned()]
    );
    assert_eq!(report.metrics.flags.f1, Some(0.6667));
    assert_eq!(report.metrics.flag_grammar.f1, Some(0.6667));
    assert_eq!(report.metrics.positionals.f1, Some(0.0));
    assert_eq!(report.metrics.output_contracts.f1, Some(1.0));
    assert_eq!(report.metrics.preconditions.f1, Some(0.0));
    assert_eq!(report.provenance.command_evidence.rate, Some(0.6667));
    assert_eq!(report.provenance.flag_evidence.rate, Some(0.5));
    assert_eq!(report.overall.metrics_scored, 7);
    assert_eq!(report.overall.score, Some(57.1));
}
