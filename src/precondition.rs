use serde::Serialize;

use crate::evidence::ProcessStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreconditionKind {
    AuthRequired,
}

impl PreconditionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::AuthRequired => "auth_required",
        }
    }
}

pub fn classify_process(
    status: &ProcessStatus,
    stdout: Option<&str>,
    stderr: Option<&str>,
) -> Option<PreconditionKind> {
    if !matches!(status, ProcessStatus::Exited { code: Some(code) } if *code != 0) {
        return None;
    }

    let text = format!(
        "{}\n{}",
        stdout.unwrap_or_default(),
        stderr.unwrap_or_default()
    )
    .to_ascii_lowercase();

    let auth_phrases = [
        "requires login",
        "login required",
        "please login",
        "please log in",
        "not logged in",
        "requires authentication",
        "authentication required",
        "not authenticated",
        "unauthenticated",
        "auth required",
        "requires auth",
    ];

    auth_phrases
        .iter()
        .any(|phrase| text.contains(phrase))
        .then_some(PreconditionKind::AuthRequired)
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
}
