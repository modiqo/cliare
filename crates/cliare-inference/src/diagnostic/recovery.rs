use super::model::{RecoveryActionFamily, RecoveryAnalysis, RecoveryBlockKind, RecoveryQuality};

impl RecoveryAnalysis {
    pub(super) fn from_text(text: &str) -> Self {
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
    let trimmed = line.trim_start_matches(['-', '*', '>', '$']).trim_start();
    if let Some((_, command)) = trimmed.split_once("  ")
        && command.split_whitespace().next().is_some_and(command_head)
    {
        return command.trim_start();
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
