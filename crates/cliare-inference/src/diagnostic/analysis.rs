use cliare_core::process_status::ProcessStatus;

use crate::precondition::PreconditionKind;

use super::model::{
    DiagnosticAnalysis, DiagnosticDocument, PreconditionCandidate, RecoveryActionFamily,
    RecoveryAnalysis,
};

pub fn analyze_process(
    status: &ProcessStatus,
    stdout: Option<&str>,
    stderr: Option<&str>,
) -> DiagnosticAnalysis {
    let document =
        DiagnosticDocument::parse(stdout.unwrap_or_default(), stderr.unwrap_or_default());

    if !matches!(status, ProcessStatus::Exited { code: Some(code) } if *code != 0) {
        return DiagnosticAnalysis::none(document.recovery);
    }

    let Some(candidate) = classify_precondition(&document) else {
        return DiagnosticAnalysis::none(document.recovery);
    };

    DiagnosticAnalysis {
        precondition: Some(candidate.kind),
        confidence: confidence(candidate.score),
        recovery: document.recovery,
    }
}

fn classify_precondition(document: &DiagnosticDocument) -> Option<PreconditionCandidate> {
    if document.parser_rejection {
        return None;
    }

    let tokens = &document.tokens;
    let recovery = &document.recovery;

    let mut candidates = Vec::new();

    if tokens.identity > 0
        && (tokens.blocker > 0
            || recovery
                .action_families
                .contains(&RecoveryActionFamily::Authenticate))
    {
        candidates.push(PreconditionCandidate {
            kind: PreconditionKind::AuthRequired,
            score: 2.0 * tokens.identity as f64
                + tokens.blocker as f64
                + action_score(recovery, RecoveryActionFamily::Authenticate, 3.0),
        });
    }

    if recovery
        .action_families
        .contains(&RecoveryActionFamily::ChangeDirectory)
        || recovery
            .action_families
            .contains(&RecoveryActionFamily::InitializeContext)
        || tokens.local_context >= 2 && tokens.blocker > 0
    {
        candidates.push(PreconditionCandidate {
            kind: PreconditionKind::LocalContextRequired,
            score: 1.4 * tokens.local_context as f64
                + 0.8 * tokens.blocker as f64
                + action_score(recovery, RecoveryActionFamily::ChangeDirectory, 4.0)
                + action_score(recovery, RecoveryActionFamily::InitializeContext, 2.0)
                + action_score(recovery, RecoveryActionFamily::InspectExisting, 0.8),
        });
    }

    if tokens.fixture_input > 0 && tokens.blocker > 0 {
        candidates.push(PreconditionCandidate {
            kind: PreconditionKind::FixtureRequired,
            score: 2.0 * tokens.fixture_input as f64 + tokens.blocker as f64,
        });
    }

    if tokens.network >= 2 || tokens.network > 0 && tokens.blocker > 0 {
        candidates.push(PreconditionCandidate {
            kind: PreconditionKind::NetworkUnavailable,
            score: 1.8 * tokens.network as f64 + 0.8 * tokens.blocker as f64,
        });
    }

    if tokens.runtime_dependency >= 2
        || tokens.runtime_dependency > 0 && tokens.blocker > 0
        || recovery
            .action_families
            .contains(&RecoveryActionFamily::StartRuntime)
        || recovery
            .action_families
            .contains(&RecoveryActionFamily::InstallDependency)
    {
        candidates.push(PreconditionCandidate {
            kind: PreconditionKind::RuntimeDependencyUnavailable,
            score: 1.8 * tokens.runtime_dependency as f64
                + 0.8 * tokens.blocker as f64
                + action_score(recovery, RecoveryActionFamily::StartRuntime, 2.0)
                + action_score(recovery, RecoveryActionFamily::InstallDependency, 2.0),
        });
    }

    candidates.sort_by(|left, right| right.score.total_cmp(&left.score));
    let best = candidates.into_iter().next()?;

    if confidence(best.score) >= 0.65 {
        Some(best)
    } else {
        None
    }
}

fn action_score(recovery: &RecoveryAnalysis, family: RecoveryActionFamily, weight: f64) -> f64 {
    if recovery.action_families.contains(&family) {
        weight
    } else {
        0.0
    }
}

fn confidence(score: f64) -> f64 {
    let confidence = score / (score + 1.5);
    (confidence * 1000.0).round() / 1000.0
}
