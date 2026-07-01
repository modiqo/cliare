use serde::Serialize;

use crate::precondition::PreconditionKind;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticAnalysis {
    pub precondition: Option<PreconditionKind>,
    pub confidence: f64,
    pub recovery: RecoveryAnalysis,
}

impl DiagnosticAnalysis {
    pub(super) fn none(recovery: RecoveryAnalysis) -> Self {
        Self {
            precondition: None,
            confidence: 0.0,
            recovery,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoveryAnalysis {
    pub quality: RecoveryQuality,
    pub labeled_blocks: Vec<RecoveryBlockKind>,
    pub action_families: Vec<RecoveryActionFamily>,
    pub command_examples: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryQuality {
    None,
    Mentioned,
    Actionable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryBlockKind {
    Fix,
    Hint,
    Note,
    Help,
    NextStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryActionFamily {
    ChangeDirectory,
    Authenticate,
    InitializeContext,
    Configure,
    InspectExisting,
    StartRuntime,
    InstallDependency,
    OtherCommand,
}

#[derive(Debug)]
pub(super) struct DiagnosticDocument {
    pub(super) tokens: TokenFeatures,
    pub(super) recovery: RecoveryAnalysis,
}

impl DiagnosticDocument {
    pub(super) fn parse(stdout: &str, stderr: &str) -> Self {
        let text = if stdout.is_empty() {
            stderr.to_owned()
        } else if stderr.is_empty() {
            stdout.to_owned()
        } else {
            format!("{stdout}\n{stderr}")
        };

        Self {
            tokens: TokenFeatures::from_text(&text),
            recovery: RecoveryAnalysis::from_text(&text),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct TokenFeatures {
    pub(super) identity: usize,
    pub(super) local_context: usize,
    pub(super) fixture_input: usize,
    pub(super) network: usize,
    pub(super) runtime_dependency: usize,
    pub(super) blocker: usize,
}

#[derive(Debug)]
pub(super) struct PreconditionCandidate {
    pub(super) kind: PreconditionKind,
    pub(super) score: f64,
}
