use serde::Serialize;

use crate::evidence::ProcessStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreconditionKind {
    AuthRequired,
    LocalContextRequired,
    FixtureRequired,
    NetworkUnavailable,
    RuntimeDependencyUnavailable,
}

impl PreconditionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::AuthRequired => "auth_required",
            Self::LocalContextRequired => "local_context_required",
            Self::FixtureRequired => "fixture_required",
            Self::NetworkUnavailable => "network_unavailable",
            Self::RuntimeDependencyUnavailable => "runtime_dependency_unavailable",
        }
    }
}

pub fn classify_process(
    status: &ProcessStatus,
    stdout: Option<&str>,
    stderr: Option<&str>,
) -> Option<PreconditionKind> {
    crate::diagnostic::analyze_process(status, stdout, stderr).precondition
}

pub fn classify_text(text: &str) -> Option<PreconditionKind> {
    classify_process(&ProcessStatus::Exited { code: Some(1) }, Some(text), None)
}

#[cfg(test)]
mod tests {
    use super::{PreconditionKind, classify_text};

    #[test]
    fn detects_auth_required_diagnostics() {
        assert_eq!(
            classify_text("error: rote requires login\nrun rote login"),
            Some(PreconditionKind::AuthRequired)
        );
        assert_eq!(
            classify_text("Authentication required before using this command"),
            Some(PreconditionKind::AuthRequired)
        );
    }

    #[test]
    fn leaves_ordinary_usage_errors_unclassified() {
        assert_eq!(classify_text("error: unknown command"), None);
        assert_eq!(classify_text("error: invalid flag --wat"), None);
    }

    #[test]
    fn detects_network_and_runtime_dependency_preconditions() {
        assert_eq!(
            classify_text("GET https://api.github.com: 403 API rate limit exceeded"),
            Some(PreconditionKind::NetworkUnavailable)
        );
        assert_eq!(
            classify_text("error connecting to api.example.com\ncheck your internet connection"),
            Some(PreconditionKind::NetworkUnavailable)
        );
        assert_eq!(
            classify_text("Cannot connect to the Docker daemon at unix:///var/run/docker.sock"),
            Some(PreconditionKind::RuntimeDependencyUnavailable)
        );
    }

    #[test]
    fn detects_local_context_preconditions() {
        assert_eq!(
            classify_text(
                "error: not in a workspace directory\n\nFix:\n  tool init demo\n  cd workspaces/demo"
            ),
            Some(PreconditionKind::LocalContextRequired)
        );
        assert_eq!(
            classify_text("failed to run git: fatal: not a git repository: .git"),
            Some(PreconditionKind::LocalContextRequired)
        );
    }

    #[test]
    fn detects_fixture_required_preconditions() {
        assert_eq!(
            classify_text("owner is required when not running interactively"),
            Some(PreconditionKind::FixtureRequired)
        );
        assert_eq!(
            classify_text("missing required argument <PROJECT_ID>"),
            Some(PreconditionKind::FixtureRequired)
        );
    }
}
