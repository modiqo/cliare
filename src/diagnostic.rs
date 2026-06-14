use serde::Serialize;

use crate::evidence::ProcessStatus;
use crate::precondition::PreconditionKind;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticAnalysis {
    pub precondition: Option<PreconditionKind>,
    pub confidence: f64,
    pub recovery: RecoveryAnalysis,
}

impl DiagnosticAnalysis {
    fn none(recovery: RecoveryAnalysis) -> Self {
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
struct DiagnosticDocument {
    tokens: TokenFeatures,
    recovery: RecoveryAnalysis,
}

#[derive(Debug, Default)]
struct TokenFeatures {
    identity: usize,
    local_context: usize,
    fixture_input: usize,
    network: usize,
    runtime_dependency: usize,
    blocker: usize,
}

#[derive(Debug)]
struct PreconditionCandidate {
    kind: PreconditionKind,
    score: f64,
}

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

impl DiagnosticDocument {
    fn parse(stdout: &str, stderr: &str) -> Self {
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

fn classify_precondition(document: &DiagnosticDocument) -> Option<PreconditionCandidate> {
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

impl TokenFeatures {
    fn from_text(text: &str) -> Self {
        let mut features = Self::default();
        let tokens = tokenize(text).collect::<Vec<_>>();

        for token in &tokens {
            match token_class(token) {
                Some(TokenClass::Identity) => features.identity += 1,
                Some(TokenClass::LocalContext) => features.local_context += 1,
                Some(TokenClass::FixtureInput) => features.fixture_input += 1,
                Some(TokenClass::Network) => features.network += 1,
                Some(TokenClass::RuntimeDependency) => features.runtime_dependency += 1,
                Some(TokenClass::Blocker) => features.blocker += 1,
                None => {}
            }
        }
        features.fixture_input += required_subject_count(&tokens);

        features
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenClass {
    Identity,
    LocalContext,
    FixtureInput,
    Network,
    RuntimeDependency,
    Blocker,
}

fn token_class(token: &str) -> Option<TokenClass> {
    match token {
        "auth" | "authenticate" | "authenticated" | "authentication" | "unauthenticated"
        | "login" | "logged" | "credential" | "credentials" | "token" | "tokens" => {
            Some(TokenClass::Identity)
        }
        "workspace" | "workspaces" | "project" | "projects" | "repository" | "repositories"
        | "repo" | "repos" | "worktree" | "directory" | "directories" | "root" | "cwd"
        | "config" | "configuration" | "git" => Some(TokenClass::LocalContext),
        "argument" | "arguments" | "option" | "options" | "flag" | "flags" | "parameter"
        | "parameters" | "field" | "fields" | "value" | "values" | "input" | "inputs"
        | "operand" | "operands" | "number" | "identifier" | "id" | "name" => {
            Some(TokenClass::FixtureInput)
        }
        "network" | "host" | "dns" | "connection" | "connect" | "resolve" | "resolved"
        | "timeout" | "timed" | "rate" | "limit" | "unreachable" => Some(TokenClass::Network),
        "daemon" | "service" | "services" | "container" | "containers" | "socket" | "runtime"
        | "dependency" | "dependencies" | "prerequisite" | "executable" | "binary" | "docker" => {
            Some(TokenClass::RuntimeDependency)
        }
        "required" | "requires" | "require" | "missing" | "not" | "no" | "cannot" | "failed"
        | "failure" | "unavailable" | "refused" | "denied" | "found" | "outside" | "inside"
        | "exceeded" | "set" | "provide" | "provided" | "supply" | "supplied" | "configure"
        | "configured" | "export" => Some(TokenClass::Blocker),
        _ => None,
    }
}

fn required_subject_count(tokens: &[String]) -> usize {
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            unknown_required_subject(token)
                && (tokens
                    .get(index + 1)
                    .is_some_and(|next| next == "required" || next == "requires")
                    || tokens
                        .get(index + 1)
                        .is_some_and(|next| next == "is" || next == "are")
                        && tokens.get(index + 2).is_some_and(|next| next == "required"))
        })
        .count()
}

fn unknown_required_subject(token: &str) -> bool {
    token.len() > 1
        && token_class(token).is_none()
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !matches!(
            token,
            "the" | "a" | "an" | "this" | "that" | "it" | "when" | "not" | "set"
        )
}

fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|token| {
            let normalized = token.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
}

impl RecoveryAnalysis {
    fn from_text(text: &str) -> Self {
        let mut labeled_blocks = Vec::new();
        let mut action_families = Vec::new();
        let mut command_examples = 0_usize;

        for line in text.lines() {
            if let Some(kind) = recovery_block_kind(line) {
                push_unique(&mut labeled_blocks, kind);
            }

            for example in command_examples_from_line(line) {
                command_examples += 1;
                let family = recovery_action_family(&example);
                push_unique(&mut action_families, family);
            }
        }

        let quality = if command_examples > 0
            || labeled_blocks
                .iter()
                .any(|kind| matches!(kind, RecoveryBlockKind::Fix | RecoveryBlockKind::NextStep))
        {
            RecoveryQuality::Actionable
        } else if labeled_blocks.is_empty() {
            RecoveryQuality::None
        } else {
            RecoveryQuality::Mentioned
        };

        Self {
            quality,
            labeled_blocks,
            action_families,
            command_examples,
        }
    }
}

fn recovery_block_kind(line: &str) -> Option<RecoveryBlockKind> {
    let trimmed = line.trim_start();
    let (label, rest) = trimmed.split_once(':')?;
    if label.len() > 24
        || label.is_empty()
        || label
            .chars()
            .any(|ch| !(ch.is_ascii_alphabetic() || ch == ' '))
    {
        return None;
    }
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }

    match label.to_ascii_lowercase().as_str() {
        "fix" | "remedy" | "resolution" => Some(RecoveryBlockKind::Fix),
        "hint" | "tip" => Some(RecoveryBlockKind::Hint),
        "note" | "info" | "warning" => Some(RecoveryBlockKind::Note),
        "help" | "docs" | "documentation" | "reference" => Some(RecoveryBlockKind::Help),
        "next" | "next step" | "next steps" => Some(RecoveryBlockKind::NextStep),
        _ => None,
    }
}

fn command_examples_from_line(line: &str) -> Vec<Vec<String>> {
    let mut examples = Vec::new();
    let trimmed = line.trim();

    if line_looks_like_command_example(line)
        && let Some(tokens) = command_tokens(trim_shell_prefix(trimmed), CommandExampleSource::Line)
    {
        examples.push(tokens);
    }

    for quoted in quoted_segments(trimmed) {
        if let Some(tokens) = command_tokens(quoted, CommandExampleSource::Quoted) {
            examples.push(tokens);
        }
    }

    if let Some(command) = inline_command_after_colon(trimmed)
        && let Some(tokens) = command_tokens(command, CommandExampleSource::Line)
    {
        examples.push(tokens);
    }

    examples
}

fn line_looks_like_command_example(line: &str) -> bool {
    let leading_whitespace = line
        .chars()
        .take_while(|ch| ch.is_ascii_whitespace())
        .count();
    let trimmed = line.trim_start();
    leading_whitespace >= 2
        || trimmed.starts_with("$ ")
        || trimmed.starts_with("> ")
        || trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
}

fn trim_shell_prefix(line: &str) -> &str {
    let trimmed = line
        .trim_start_matches(|ch: char| ch == '-' || ch == '*' || ch == '>' || ch == '$')
        .trim_start();
    if let Some((_, command)) = trimmed.split_once("  ") {
        if command.split_whitespace().next().is_some_and(command_head) {
            return command.trim_start();
        }
    }
    trimmed
}

fn inline_command_after_colon(line: &str) -> Option<&str> {
    let (_, tail) = line.rsplit_once(':')?;
    let leading_whitespace = tail
        .chars()
        .take_while(|ch| ch.is_ascii_whitespace())
        .count();
    if leading_whitespace >= 2 || tail.trim_start().starts_with("$ ") {
        Some(tail.trim_start())
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
enum CommandExampleSource {
    Line,
    Quoted,
}

fn command_tokens(line: &str, source: CommandExampleSource) -> Option<Vec<String>> {
    let cleaned = line
        .split('#')
        .next()
        .unwrap_or(line)
        .trim()
        .trim_matches(|ch: char| ch == '\'' || ch == '"' || ch == '`');
    let tokens = cleaned
        .split_whitespace()
        .map(|token| token.trim_matches(|ch: char| ch == '\'' || ch == '"' || ch == '`'))
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();

    let head = tokens.first()?;
    if head.starts_with('-') {
        return None;
    }
    if matches!(source, CommandExampleSource::Quoted) && tokens.len() < 2 && head != "cd" {
        return None;
    }
    if command_head(head) {
        Some(tokens)
    } else {
        None
    }
}

fn command_head(token: &str) -> bool {
    token == "cd"
        || token == "open"
        || token == "brew"
        || token == "docker"
        || token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/'))
            && token.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn quoted_segments(line: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    for quote in ['`', '\'', '"'] {
        let mut start = None;
        for (index, ch) in line.char_indices() {
            if ch == quote {
                if let Some(open) = start.take() {
                    if open < index {
                        segments.push(&line[open + quote.len_utf8()..index]);
                    }
                } else {
                    start = Some(index);
                }
            }
        }
    }
    segments
}

fn recovery_action_family(tokens: &[String]) -> RecoveryActionFamily {
    if tokens.first().is_some_and(|token| token == "cd") {
        return RecoveryActionFamily::ChangeDirectory;
    }

    let normalized = tokens
        .iter()
        .map(|token| {
            token
                .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if normalized
        .iter()
        .any(|token| token == "login" || token == "auth")
    {
        return RecoveryActionFamily::Authenticate;
    }
    if normalized.iter().any(|token| {
        matches!(
            token.as_str(),
            "init" | "initialize" | "setup" | "bootstrap" | "link"
        )
    }) {
        return RecoveryActionFamily::InitializeContext;
    }
    if normalized
        .iter()
        .any(|token| token == "config" || token == "configure")
    {
        return RecoveryActionFamily::Configure;
    }
    if normalized
        .iter()
        .any(|token| token == "ls" || token == "list" || token == "show")
    {
        return RecoveryActionFamily::InspectExisting;
    }
    if normalized
        .iter()
        .any(|token| token == "start" || token == "restart")
    {
        return RecoveryActionFamily::StartRuntime;
    }
    if normalized
        .iter()
        .any(|token| token == "install" || token == "add")
    {
        return RecoveryActionFamily::InstallDependency;
    }

    RecoveryActionFamily::OtherCommand
}

fn push_unique<T>(items: &mut Vec<T>, item: T)
where
    T: Copy + Eq,
{
    if !items.contains(&item) {
        items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::{RecoveryActionFamily, RecoveryBlockKind, RecoveryQuality, analyze_process};
    use crate::evidence::ProcessStatus;
    use crate::precondition::PreconditionKind;

    #[test]
    fn classifies_workspace_context_from_structured_recovery() {
        let text = "error: not in a workspace directory\n\n  Fix:\n    rote init workflow-name --seq\n    cd ~/.rote/workspaces/workflow-name\n\nhint: or list existing: 'rote ls'\n";
        let analysis = analyze_process(&ProcessStatus::Exited { code: Some(1) }, None, Some(text));

        assert_eq!(
            analysis.precondition,
            Some(PreconditionKind::LocalContextRequired)
        );
        assert_eq!(analysis.recovery.quality, RecoveryQuality::Actionable);
        assert!(
            analysis
                .recovery
                .labeled_blocks
                .contains(&RecoveryBlockKind::Fix)
        );
        assert!(
            analysis
                .recovery
                .action_families
                .contains(&RecoveryActionFamily::ChangeDirectory)
        );
    }

    #[test]
    fn classifies_repository_context_without_exact_phrase_matching() {
        let text = "failed to run git: fatal: not a git repository (or any parent): .git";
        let analysis = analyze_process(&ProcessStatus::Exited { code: Some(1) }, None, Some(text));

        assert_eq!(
            analysis.precondition,
            Some(PreconditionKind::LocalContextRequired)
        );
        assert_eq!(analysis.recovery.quality, RecoveryQuality::None);
    }

    #[test]
    fn classifies_token_environment_variable_as_auth_required() {
        let text = "gh: To use GitHub CLI in automation, set the GH_TOKEN environment variable.";
        let analysis = analyze_process(&ProcessStatus::Exited { code: Some(4) }, None, Some(text));

        assert_eq!(analysis.precondition, Some(PreconditionKind::AuthRequired));
        assert_eq!(analysis.recovery.quality, RecoveryQuality::None);
    }

    #[test]
    fn classifies_inline_auth_command_recovery() {
        let text = "To get started with GitHub CLI, please run:  gh auth login -s codespace";
        let analysis = analyze_process(&ProcessStatus::Exited { code: Some(4) }, None, Some(text));

        assert_eq!(analysis.precondition, Some(PreconditionKind::AuthRequired));
        assert_eq!(analysis.recovery.quality, RecoveryQuality::Actionable);
        assert!(
            analysis
                .recovery
                .action_families
                .contains(&RecoveryActionFamily::Authenticate)
        );
    }

    #[test]
    fn classifies_missing_fixture_inputs_without_treating_output_as_malformed() {
        let analysis = analyze_process(
            &ProcessStatus::Exited { code: Some(1) },
            None,
            Some("owner is required when not running interactively"),
        );

        assert_eq!(
            analysis.precondition,
            Some(PreconditionKind::FixtureRequired)
        );
        assert_eq!(analysis.recovery.quality, RecoveryQuality::None);

        let analysis = analyze_process(
            &ProcessStatus::Exited { code: Some(2) },
            None,
            Some("missing required argument <PROJECT_ID>"),
        );

        assert_eq!(
            analysis.precondition,
            Some(PreconditionKind::FixtureRequired)
        );

        let analysis = analyze_process(
            &ProcessStatus::Exited { code: Some(1) },
            None,
            Some("required flag(s) \"title\" not set"),
        );

        assert_eq!(
            analysis.precondition,
            Some(PreconditionKind::FixtureRequired)
        );
    }

    #[test]
    fn keeps_plain_usage_errors_unclassified() {
        let analysis = analyze_process(
            &ProcessStatus::Exited { code: Some(2) },
            None,
            Some("error: unexpected argument '--wat'"),
        );

        assert_eq!(analysis.precondition, None);
        assert_eq!(analysis.recovery.quality, RecoveryQuality::None);
    }
}
