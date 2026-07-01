use super::*;
use crate::report_format::shell_arg;

pub(super) fn persona_priority(persona: Persona, category: ActionCategory) -> u16 {
    match persona {
        Persona::Maintainer => match category {
            ActionCategory::Output => 10,
            ActionCategory::Discovery => 20,
            ActionCategory::Grammar => 30,
            ActionCategory::Recovery => 40,
            ActionCategory::Safety => 50,
            _ => 60,
        },
        Persona::Harness => match category {
            ActionCategory::Safety => 10,
            ActionCategory::Output => 20,
            ActionCategory::Discovery => 30,
            ActionCategory::Grammar => 40,
            _ => 60,
        },
        Persona::Platform => match category {
            ActionCategory::Policy => 10,
            ActionCategory::Safety => 20,
            ActionCategory::Coverage => 30,
            ActionCategory::Output => 40,
            _ => 60,
        },
        Persona::Security => match category {
            ActionCategory::Safety => 10,
            ActionCategory::Discovery => 20,
            ActionCategory::Coverage => 30,
            _ => 70,
        },
        Persona::Oss => match category {
            ActionCategory::Publishing => 10,
            ActionCategory::Output => 20,
            ActionCategory::Discovery => 30,
            ActionCategory::Coverage => 40,
            _ => 70,
        },
        Persona::Devrel => match category {
            ActionCategory::Publishing => 10,
            ActionCategory::Output => 20,
            ActionCategory::Discovery => 30,
            ActionCategory::Calibration => 40,
            _ => 70,
        },
        Persona::Research => match category {
            ActionCategory::Calibration => 10,
            ActionCategory::Coverage => 20,
            ActionCategory::Discovery => 30,
            _ => 70,
        },
    }
}

pub(super) fn run_recommendations(
    persona: Persona,
    scorecard: &ScorecardArtifact,
    artifact_dir: &Path,
) -> Vec<RunRecommendation> {
    let coverage = &scorecard.coverage;
    let target = shell_arg(&scorecard.target.requested.display().to_string());
    let out = shell_arg(&artifact_dir.display().to_string());
    let mut recommendations = Vec::new();

    if !coverage.traversal_complete {
        let depth = if coverage.observed_max_depth >= coverage.max_depth {
            coverage.max_depth + 2
        } else {
            coverage.max_depth.max(8)
        };
        let probes = if coverage.budget_exhausted {
            coverage.max_probes.saturating_mul(2).max(1_000)
        } else {
            coverage.max_probes.max(1_000)
        };
        recommendations.push(RunRecommendation {
            id: "run.deepen_surface".to_owned(),
            priority: 10,
            command: format!(
                "cliare measure {target} --out {out} --profile deep --max-depth {depth} --max-probes {probes} --concurrency {} --refresh",
                coverage.concurrency_limit.max(8)
            ),
            purpose: "Expand command-surface coverage before treating this run as complete."
                .to_owned(),
            when_to_use: "Use when traversal is incomplete, depth was exhausted, or the probe frontier still has pending work."
                .to_owned(),
        });
    }

    if coverage.machine_readable_output_contracts == 0 {
        recommendations.push(RunRecommendation {
            id: "run.after_output_contracts".to_owned(),
            priority: 30,
            command: format!("cliare measure {target} --out {out} --profile standard --refresh"),
            purpose:
                "Re-measure after adding JSON or YAML output modes to read/list/show commands."
                    .to_owned(),
            when_to_use: "Use after improving machine-readable output contracts.".to_owned(),
        });
    }

    match persona {
        Persona::Platform => recommendations.push(RunRecommendation {
            id: "run.platform_guard".to_owned(),
            priority: 20,
            command: format!(
                "cliare guard {target} --baseline .cliare/baseline.scorecard.json --policy cliare.policy.json --out {out}"
            ),
            purpose: "Turn the measurement into a release gate with score and policy checks."
                .to_owned(),
            when_to_use: "Use in CI after selecting policy thresholds for the organization."
                .to_owned(),
        }),
        Persona::Security => recommendations.push(RunRecommendation {
            id: "run.security_packet".to_owned(),
            priority: 20,
            command: format!("cliare report security --out {out} --write"),
            purpose: "Persist a security-focused packet for approval review.".to_owned(),
            when_to_use: "Use whenever side effects, auth gates, or agent exposure approvals are being reviewed."
                .to_owned(),
        }),
        Persona::Harness => recommendations.push(RunRecommendation {
            id: "run.harness_json".to_owned(),
            priority: 20,
            command: format!("cliare report harness --out {out} --format json --write"),
            purpose: "Persist a machine-readable packet for tool routers and harness policy."
                .to_owned(),
            when_to_use: "Use before exposing a CLI subset to agents.".to_owned(),
        }),
        Persona::Oss | Persona::Devrel => recommendations.push(RunRecommendation {
            id: "run.publishable_standard".to_owned(),
            priority: 20,
            command: format!("cliare measure {target} --out {out} --profile standard --refresh"),
            purpose: "Refresh a publishable local scorecard before release communication."
                .to_owned(),
            when_to_use: "Use before adding badges, release notes, or public scorecard artifacts."
                .to_owned(),
        }),
        Persona::Research => recommendations.push(RunRecommendation {
            id: "run.research_deep".to_owned(),
            priority: 20,
            command: format!(
                "cliare measure {target} --out {out} --profile deep --max-depth 8 --max-probes 1000 --concurrency 8 --refresh"
            ),
            purpose: "Produce a deeper evidence set suitable for labeling and calibration review."
                .to_owned(),
            when_to_use: "Use when adding the target to a benchmark corpus or truth-set workflow."
                .to_owned(),
        }),
        Persona::Maintainer => {}
    }

    recommendations.sort_by_key(|item| item.priority);
    recommendations
}

pub(super) fn notes(persona: Persona, scorecard: &ScorecardArtifact) -> Vec<OutcomeNote> {
    let mut notes = Vec::new();
    notes.push(OutcomeNote {
        level: "info",
        text: "Persona packets are projections over measured artifacts; they do not rerun the target CLI."
            .to_owned(),
    });
    notes.push(OutcomeNote {
        level: "info",
        text: "Black-box measurement reports observed evidence and uncertainty; it cannot prove that hidden command surface does not exist."
            .to_owned(),
    });
    if scorecard.score.status == "experimental_partial" {
        notes.push(OutcomeNote {
            level: "warning",
            text: "Score v0 is suitable for CI feedback and improvement tracking, not certified public ranking."
                .to_owned(),
        });
    }
    match persona {
        Persona::Security => notes.push(OutcomeNote {
            level: "warning",
            text: if scorecard.coverage.side_effect_files_total > 0 {
                "Observed side effects require review before approval; inspect evidence paths, fixture state, auth state, and traversal completeness."
            } else {
                "Absence of observed side effects is not an approval by itself; review profile, fixtures, auth state, and traversal completeness."
            }
            .to_owned(),
        }),
        Persona::Oss | Persona::Devrel => notes.push(OutcomeNote {
            level: "warning",
            text: "Public claims should distinguish local scorecards from future certified leaderboard entries."
                .to_owned(),
        }),
        Persona::Research => notes.push(OutcomeNote {
            level: "info",
            text: "Use evidence IDs, model versions, budgets, and binary fingerprint when citing or labeling this run."
                .to_owned(),
        }),
        _ => {}
    }
    notes
}
