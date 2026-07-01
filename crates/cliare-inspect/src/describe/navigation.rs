use super::model::{ArtifactHealth, ArtifactKind, ArtifactSummaries, NavigationStep};

pub(super) fn health(missing_required: &[String], summaries: &ArtifactSummaries) -> ArtifactHealth {
    let mut warnings = Vec::new();
    if !missing_required.is_empty() {
        warnings.push(format!(
            "{} required artifact(s) are missing",
            missing_required.len()
        ));
    }
    if summaries.scorecard.is_none()
        && summaries.benchmark.is_none()
        && summaries.contexts.is_none()
    {
        warnings.push("no scorecard.json or benchmark.json summary could be parsed".to_owned());
    }
    if summaries
        .job
        .as_ref()
        .is_some_and(|job| job.status == "failed")
    {
        warnings.push("latest measurement job failed".to_owned());
    }
    ArtifactHealth {
        status: if warnings.is_empty() {
            "ok".to_owned()
        } else {
            "attention".to_owned()
        },
        warnings,
    }
}

pub(super) fn navigation_plan(kind: ArtifactKind) -> Vec<NavigationStep> {
    match kind {
        ArtifactKind::Measurement | ArtifactKind::Mixed => vec![
            step(
                1,
                "scorecard.json",
                "Read posture, score, coverage pressure, and provenance.",
            ),
            step(2, "issues.json", "Use as the canonical remediation queue."),
            step(
                3,
                "command-index.json",
                "Choose commands using runtime state, preconditions, parameters, and suitability.",
            ),
            step(
                4,
                "persona-<persona>.md",
                "Read the role-specific brief before drilling down.",
            ),
            step(
                5,
                "evidence.jsonl",
                "Verify claims by event id before making strong conclusions.",
            ),
            step(
                6,
                "jobs/current",
                "Check whether the latest measurement is still running or failed.",
            ),
        ],
        ArtifactKind::Benchmark => vec![
            step(
                1,
                "benchmark.json",
                "Read corpus totals, calibration status, and target summaries.",
            ),
            step(
                2,
                "benchmark.md",
                "Use the human-readable benchmark report.",
            ),
            step(
                3,
                "<target>/scorecard.json",
                "Inspect individual target measurements.",
            ),
            step(
                4,
                "<target>/command-index.json",
                "Inspect target command surfaces.",
            ),
        ],
        ArtifactKind::ContextSuite => vec![
            step(
                1,
                "context-suite.json",
                "Read persisted contexts, scores, preconditions, and artifact directories.",
            ),
            step(
                2,
                "context-compare.md",
                "Use the human-readable comparison table.",
            ),
            step(
                3,
                "contexts/<name>/persona-maintainer.md",
                "Open the persona packet inside the context you want to review.",
            ),
            step(
                4,
                "contexts/<name>/command-index.json",
                "Use the command index for context-specific harness navigation.",
            ),
        ],
        ArtifactKind::Unknown => vec![
            step(1, "README.md", "Read local context if present."),
            step(
                2,
                "artifact-map.json",
                "Use the generated map after running describe --write.",
            ),
        ],
    }
}

fn step(rank: u8, path: &str, purpose: &str) -> NavigationStep {
    NavigationStep {
        rank,
        path: path.to_owned(),
        purpose: purpose.to_owned(),
    }
}
