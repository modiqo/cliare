use std::collections::BTreeMap;

use crate::output::OutputMode;

const MAX_COMMAND_PATH_SEGMENTS: usize = 3;

#[derive(Debug, Clone)]
pub struct HelpDocument {
    lines: Vec<LayoutLine>,
}

impl HelpDocument {
    pub fn parse(text: &str) -> Self {
        let lines = text
            .lines()
            .enumerate()
            .map(|(index, raw)| LayoutLine::parse(index, raw))
            .collect();
        Self { lines }
    }

    pub fn rows(&self) -> impl Iterator<Item = &LayoutLine> {
        self.lines
            .iter()
            .filter(|line| line.is_row_like() && !line.is_continuation_like())
    }

    pub fn row_blocks(&self) -> Vec<Vec<&LayoutLine>> {
        let mut blocks = Vec::new();
        let mut current = Vec::new();

        for line in &self.lines {
            if line.is_row_like() && !line.is_continuation_like() {
                current.push(line);
            } else if !current.is_empty() {
                blocks.push(current);
                current = Vec::new();
            }
        }
        if !current.is_empty() {
            blocks.push(current);
        }

        blocks
    }

    pub fn usage_lines(&self) -> impl Iterator<Item = &LayoutLine> {
        self.lines.iter().filter(|line| {
            line.text
                .split_once(':')
                .is_some_and(|(prefix, _)| prefix.eq_ignore_ascii_case("usage"))
        })
    }

    pub fn is_help_like(&self) -> bool {
        let row_count = self.rows().count();
        let header_count = self
            .lines
            .iter()
            .filter(|line| line.is_header_like())
            .count();
        let option_count = self
            .lines
            .iter()
            .filter(|line| line.tokens.iter().any(|token| token.starts_with('-')))
            .count();

        row_count >= 2 || (header_count >= 1 && row_count >= 1) || option_count >= 1
    }
}

#[derive(Debug, Clone)]
pub struct LayoutLine {
    pub index: usize,
    pub indent: usize,
    pub text: String,
    pub columns: Vec<String>,
    pub tokens: Vec<String>,
}

impl LayoutLine {
    fn parse(index: usize, raw: &str) -> Self {
        let indent = raw.chars().take_while(|ch| ch.is_whitespace()).count();
        let text = raw.trim().to_owned();
        let columns = split_columns(raw);
        let tokens = text.split_whitespace().map(str::to_owned).collect();

        Self {
            index,
            indent,
            text,
            columns,
            tokens,
        }
    }

    fn is_header_like(&self) -> bool {
        !self.text.is_empty()
            && self.indent == 0
            && self.text.ends_with(':')
            && self
                .text
                .trim_end_matches(':')
                .chars()
                .any(|ch| ch.is_ascii_alphabetic())
    }

    fn is_row_like(&self) -> bool {
        self.indent > 0 && self.columns.len() >= 2 && !self.text.is_empty()
    }

    fn is_continuation_like(&self) -> bool {
        self.columns
            .first()
            .is_some_and(|column| column.starts_with("--") && self.indent > 6)
    }
}

pub fn command_candidates(text: &str, binary_name: &str) -> Vec<CandidateCommand> {
    let document = HelpDocument::parse(text);
    let mut candidates = BTreeMap::<Vec<String>, CandidateCommand>::new();

    for block in document.row_blocks() {
        let rows = block
            .iter()
            .filter_map(|row| command_row_candidate(row, binary_name))
            .collect::<Vec<_>>();
        if !is_command_table_block(block.len(), &rows) {
            continue;
        }
        for row in rows {
            for path in &row.paths {
                let aliases = row
                    .paths
                    .iter()
                    .filter_map(|other| {
                        if other == path || other.len() != 1 {
                            None
                        } else {
                            other.first().cloned()
                        }
                    })
                    .collect::<Vec<_>>();
                candidates.entry(path.clone()).or_insert(CandidateCommand {
                    path: path.clone(),
                    aliases,
                    summary: row.summary.clone(),
                    evidence_detail: format!("layout row {}", row.index),
                });
            }
        }
    }

    candidates.into_values().collect()
}

pub fn flag_candidates(text: &str) -> Vec<CandidateFlag> {
    let document = HelpDocument::parse(text);
    let mut candidates = BTreeMap::<String, CandidateFlag>::new();

    for row in document.rows() {
        for column in &row.columns {
            for token in column.split_whitespace() {
                let Some(name) = long_flag_name(token) else {
                    continue;
                };
                let grammar = flag_grammar_in(column, row.columns.get(1).map(String::as_str));
                candidates.entry(name.clone()).or_insert(CandidateFlag {
                    name,
                    short: short_flag_in(column),
                    summary: row.columns.get(1).cloned(),
                    value_kind: grammar.value_kind,
                    value_name: grammar.value_name,
                    required: grammar.required,
                    repeatable: grammar.repeatable,
                    evidence_detail: format!("layout row {}", row.index),
                });
            }
        }
    }

    candidates.into_values().collect()
}

pub fn output_mode_candidates(text: &str) -> Vec<CandidateOutputMode> {
    let mut candidates = BTreeMap::<(OutputMode, Vec<String>), CandidateOutputMode>::new();

    for flag in flag_candidates(text) {
        for mode in output_modes_for_flag(&flag) {
            let argv_fragment = output_mode_argv_fragment(&flag, mode);
            candidates
                .entry((mode, argv_fragment.clone()))
                .or_insert_with(|| CandidateOutputMode {
                    mode,
                    flag_name: flag.name.clone(),
                    argv_fragment,
                    evidence_detail: flag.evidence_detail.clone(),
                });
        }
    }

    candidates.into_values().collect()
}

pub fn usage_arguments(
    text: &str,
    binary_name: &str,
    current_path: &[String],
) -> Vec<CandidateArgument> {
    let document = HelpDocument::parse(text);
    let mut arguments = Vec::new();

    for line in document.usage_lines() {
        let Some((_, usage)) = line.text.split_once(':') else {
            continue;
        };
        let mut tokens = usage.split_whitespace().map(clean_token).peekable();
        if tokens.peek().is_some_and(|token| *token == binary_name) {
            tokens.next();
        }
        for segment in current_path {
            if tokens.peek().is_some_and(|token| *token == segment) {
                tokens.next();
            }
        }

        let tokens = tokens.collect::<Vec<_>>();
        let mut index = 0_usize;
        while index < tokens.len() {
            let token = tokens[index];
            if token.starts_with('-') || token.contains("--") {
                if tokens
                    .get(index + 1)
                    .is_some_and(|next| value_name_from_token(next).is_some())
                {
                    index += 2;
                } else {
                    index += 1;
                }
                continue;
            }
            if let Some(argument) = usage_argument_from_token(token, line.index) {
                arguments.push(argument);
            }
            index += 1;
        }
    }

    dedup_arguments(arguments)
}

pub fn is_help_like(text: &str) -> bool {
    HelpDocument::parse(text).is_help_like()
}

pub fn is_manpage_like(text: &str) -> bool {
    text.as_bytes().windows(2).any(|window| window[1] == 0x08)
}

#[derive(Debug, Clone)]
pub struct CandidateCommand {
    pub path: Vec<String>,
    pub aliases: Vec<String>,
    pub summary: Option<String>,
    pub evidence_detail: String,
}

#[derive(Debug, Clone)]
pub struct CandidateFlag {
    pub name: String,
    pub short: Option<String>,
    pub summary: Option<String>,
    pub value_kind: CandidateFlagValueKind,
    pub value_name: Option<String>,
    pub required: bool,
    pub repeatable: bool,
    pub evidence_detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateOutputMode {
    pub mode: OutputMode,
    pub flag_name: String,
    pub argv_fragment: Vec<String>,
    pub evidence_detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateFlagValueKind {
    Boolean,
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateArgument {
    pub name: String,
    pub required: bool,
    pub variadic: bool,
    pub evidence_detail: String,
}

#[derive(Debug)]
struct CommandRowCandidate {
    index: usize,
    paths: Vec<Vec<String>>,
    summary: Option<String>,
    invocation_width: usize,
    invocation_tokens: usize,
    title_like_summary: bool,
    distinctive_invocation: bool,
}

fn command_row_candidate(row: &LayoutLine, binary_name: &str) -> Option<CommandRowCandidate> {
    let first_column = row.columns.first()?;
    let paths = command_paths_from_cell(first_column, binary_name);
    if paths.is_empty() {
        return None;
    }
    let distinctive_invocation = paths.iter().flatten().any(|segment| {
        segment
            .chars()
            .any(|ch| !ch.is_ascii_lowercase() || matches!(ch, '-' | '_' | ':' | '@'))
    });
    Some(CommandRowCandidate {
        index: row.index,
        paths,
        summary: row.columns.get(1).cloned(),
        invocation_width: first_column.chars().count(),
        invocation_tokens: command_prefix_token_count(first_column, binary_name),
        title_like_summary: row
            .columns
            .get(1)
            .is_some_and(|summary| starts_title_like(summary)),
        distinctive_invocation,
    })
}

fn is_command_table_block(row_count: usize, candidates: &[CommandRowCandidate]) -> bool {
    if candidates.len() == 1 {
        let row = &candidates[0];
        return row.invocation_width <= 48
            && row.invocation_tokens <= 3
            && (row.paths.len() > 1 || row.title_like_summary || row.distinctive_invocation);
    }
    if candidates.len() < 2 {
        return false;
    }

    let density = candidates.len() as f64 / row_count.max(1) as f64;
    let compact = candidates
        .iter()
        .filter(|row| row.invocation_width <= 48 && row.invocation_tokens <= 3)
        .count();
    let compact_ratio = compact as f64 / candidates.len() as f64;
    let strong = candidates
        .iter()
        .filter(|row| row.title_like_summary || row.distinctive_invocation)
        .count();
    let strong_ratio = strong as f64 / candidates.len() as f64;
    density >= 0.6 && compact_ratio >= 0.8 && strong_ratio >= 0.5
}

fn starts_title_like(text: &str) -> bool {
    text.chars()
        .find(|ch| !ch.is_whitespace())
        .is_some_and(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn command_paths_from_cell(cell: &str, binary_name: &str) -> Vec<Vec<String>> {
    let mut path = Vec::new();
    let mut saw_alias_separator = false;

    for token in cell.split_whitespace() {
        let cleaned = clean_token(token);
        if cleaned.is_empty() {
            continue;
        }
        if cleaned == binary_name && path.is_empty() {
            continue;
        }
        if cleaned.chars().all(|ch| ch.is_ascii_digit()) {
            return Vec::new();
        }
        if is_argument_like(cleaned) || cleaned.starts_with('-') {
            break;
        }
        if !is_command_token(cleaned) {
            break;
        }
        if path.len() >= MAX_COMMAND_PATH_SEGMENTS {
            return Vec::new();
        }
        saw_alias_separator |= token.ends_with(',');
        path.push(cleaned.to_owned());
    }

    if path.is_empty() {
        return Vec::new();
    }
    if saw_alias_separator {
        return path.into_iter().map(|segment| vec![segment]).collect();
    }

    vec![path]
}

fn command_prefix_token_count(cell: &str, binary_name: &str) -> usize {
    command_paths_from_cell(cell, binary_name)
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
}

fn is_argument_like(token: &str) -> bool {
    token.starts_with('<')
        || token.starts_with('[')
        || token.contains('<')
        || token.contains('[')
        || token.contains('=')
        || token == "..."
}

fn is_command_token(token: &str) -> bool {
    token
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '@'))
        && !is_title_case_word(token)
}

fn is_title_case_word(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_uppercase())
        && chars.any(|ch| ch.is_ascii_lowercase())
        && token.chars().all(|ch| ch.is_ascii_alphabetic())
}

fn short_flag_in(text: &str) -> Option<String> {
    text.split(|ch: char| ch == ',' || ch.is_whitespace())
        .map(clean_token)
        .find(|token| {
            token.starts_with('-')
                && !token.starts_with("--")
                && token.len() > 1
                && token.chars().skip(1).all(|ch| ch.is_ascii_alphabetic())
        })
        .map(str::to_owned)
}

fn long_flag_name(token: &str) -> Option<String> {
    let cleaned = clean_token(token);
    if !cleaned.starts_with("--") {
        return None;
    }
    let end = cleaned.find(['=', '[', '<']).unwrap_or(cleaned.len());
    let name = &cleaned[..end];
    if name.len() > 2 {
        Some(name.to_owned())
    } else {
        None
    }
}

#[derive(Debug)]
struct CandidateFlagGrammar {
    value_kind: CandidateFlagValueKind,
    value_name: Option<String>,
    required: bool,
    repeatable: bool,
}

fn flag_grammar_in(invocation: &str, summary: Option<&str>) -> CandidateFlagGrammar {
    let tokens = invocation
        .split_whitespace()
        .map(clean_token)
        .collect::<Vec<_>>();
    let mut value_kind = CandidateFlagValueKind::Boolean;
    let mut value_name = None;
    let mut repeatable = invocation.contains("...");

    for (index, token) in tokens.iter().enumerate() {
        if !token.starts_with("--") {
            continue;
        }
        if token.contains("[=<") || token.contains("[=") {
            value_kind = CandidateFlagValueKind::Optional;
            value_name = value_name_from_token(token);
        } else if token.contains("=<") || token.contains('=') || token.contains('<') {
            value_kind = CandidateFlagValueKind::Required;
            value_name = value_name_from_token(token);
        } else if let Some(next) = tokens.get(index + 1) {
            if next.starts_with("[<") {
                value_kind = CandidateFlagValueKind::Optional;
                value_name = value_name_from_token(next);
                repeatable |= next.contains("...");
            } else if next.starts_with('<') {
                value_kind = CandidateFlagValueKind::Required;
                value_name = value_name_from_token(next);
                repeatable |= next.contains("...");
            }
        }
    }

    let required = summary.is_some_and(|text| {
        text.split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|word| word.eq_ignore_ascii_case("required"))
    });
    repeatable |= summary.is_some_and(|text| {
        text.split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|word| {
                word.eq_ignore_ascii_case("repeatable")
                    || word.eq_ignore_ascii_case("multiple")
                    || word.eq_ignore_ascii_case("many")
            })
    });

    CandidateFlagGrammar {
        value_kind,
        value_name,
        required,
        repeatable,
    }
}

fn usage_argument_from_token(token: &str, line_index: usize) -> Option<CandidateArgument> {
    if token.starts_with('-')
        || token.contains("--")
        || !token.contains('<') && !token.contains('[')
    {
        return None;
    }
    let name = value_name_from_token(token)?;
    if is_generic_usage_placeholder(&name) {
        return None;
    }

    Some(CandidateArgument {
        name,
        required: !token.starts_with('['),
        variadic: token.contains("..."),
        evidence_detail: format!("usage line {}", line_index),
    })
}

fn value_name_from_token(token: &str) -> Option<String> {
    let start = token.find('<').or_else(|| token.find('['))?;
    let mut value = token[start..].trim_matches(|ch: char| {
        matches!(
            ch,
            '<' | '>' | '[' | ']' | '=' | ',' | ':' | ';' | '(' | ')' | '{' | '}' | '.'
        )
    });
    if let Some(stripped) = value.strip_prefix('=') {
        value = stripped;
    }
    let end = value.find(['>', ']', '.', '=', ',']).unwrap_or(value.len());
    let name = value[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_ascii_lowercase().replace('-', "_"))
    }
}

fn is_generic_usage_placeholder(name: &str) -> bool {
    matches!(
        name,
        "options" | "option" | "command" | "commands" | "subcommand" | "subcommands" | "args"
    )
}

fn output_modes_for_flag(flag: &CandidateFlag) -> Vec<OutputMode> {
    let mut modes = Vec::new();
    let flag_name = flag.name.trim_start_matches("--").replace(['-', '_'], " ");
    let value_name = flag.value_name.as_deref().unwrap_or("").replace('_', " ");
    let summary = flag.summary.as_deref().unwrap_or("");
    let haystack = format!("{flag_name} {value_name} {summary}").to_ascii_lowercase();

    for (needle, mode) in [
        ("json", OutputMode::Json),
        ("yaml", OutputMode::Yaml),
        ("yml", OutputMode::Yaml),
        ("table", OutputMode::Table),
        ("plain", OutputMode::Plain),
        ("text", OutputMode::Plain),
    ] {
        if contains_word(&haystack, needle) && !modes.contains(&mode) {
            modes.push(mode);
        }
    }

    modes
}

fn output_mode_argv_fragment(flag: &CandidateFlag, mode: OutputMode) -> Vec<String> {
    if flag.value_kind == CandidateFlagValueKind::Boolean {
        vec![flag.name.clone()]
    } else {
        vec![flag.name.clone(), mode.label().to_owned()]
    }
}

fn contains_word(text: &str, word: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|token| token == word)
}

fn dedup_arguments(arguments: Vec<CandidateArgument>) -> Vec<CandidateArgument> {
    let mut deduped = BTreeMap::<String, CandidateArgument>::new();
    for argument in arguments {
        deduped
            .entry(argument.name.clone())
            .and_modify(|existing| {
                existing.required |= argument.required;
                existing.variadic |= argument.variadic;
            })
            .or_insert(argument);
    }
    deduped.into_values().collect()
}

fn clean_token(token: &str) -> &str {
    token.trim_matches(|ch: char| matches!(ch, ',' | ':' | ';' | '(' | ')' | '{' | '}'))
}

fn split_columns(line: &str) -> Vec<String> {
    let mut columns = Vec::new();
    let mut current = String::new();
    let mut spaces = 0_usize;

    for ch in line.trim().chars() {
        if ch.is_whitespace() {
            spaces += 1;
            continue;
        }

        if spaces >= 2 && !current.trim().is_empty() {
            columns.push(current.trim().to_owned());
            current.clear();
        } else if spaces == 1 && !current.is_empty() {
            current.push(' ');
        }

        spaces = 0;
        current.push(ch);
    }

    if !current.trim().is_empty() {
        columns.push(current.trim().to_owned());
    }

    columns
}

#[cfg(test)]
mod tests {
    use super::{
        CandidateFlagValueKind, command_candidates, flag_candidates, is_help_like, is_manpage_like,
        output_mode_candidates, usage_arguments,
    };
    use crate::output::OutputMode;

    #[test]
    fn extracts_commands_from_generic_aligned_rows() {
        let text = "TOOLS:\n  workspace ls [--flat]    List workspaces\n  flow search <QUERY>       Search flows\n";
        let candidates = command_candidates(text, "rote");

        assert!(
            candidates
                .iter()
                .any(|item| item.path == ["workspace", "ls"])
        );
        assert!(
            candidates
                .iter()
                .any(|item| item.path == ["flow", "search"])
        );
    }

    #[test]
    fn treats_framework_help_as_generic_layout() {
        let text = "Commands:\n  measure  Run probes\n\nOptions:\n  -h, --help     Print help\n";

        assert!(is_help_like(text));
        assert!(
            command_candidates(text, "cliare")
                .iter()
                .any(|item| item.path == ["measure"])
        );
        assert!(
            flag_candidates(text)
                .iter()
                .any(|item| item.name == "--help")
        );
    }

    #[test]
    fn ignores_wrapped_prose_that_happens_to_align_like_columns() {
        let text = "DESCRIPTION\n       current branch  with the same name on the remote\n       be given        from the command line or configuration\n       default mode    is selected for ordinary users\n";
        let candidates = command_candidates(text, "git");

        assert!(candidates.is_empty());
    }

    #[test]
    fn rejects_overlong_invocation_prefixes_from_prose() {
        let text = "DETAILS\n       updates remote refs using local refs  while sending objects\n       pushes all matching branches at once  when configured\n";
        let candidates = command_candidates(text, "git");

        assert!(candidates.is_empty());
    }

    #[test]
    fn rejects_title_case_prose_as_command_tokens() {
        let text = "OPTIONS\n       Show colored output  depending on configuration\n       Use mailmap file     to map author names\n";
        let candidates = command_candidates(text, "git");

        assert!(candidates.is_empty());
    }

    #[test]
    fn detects_backspace_formatted_manpage_output() {
        assert!(is_manpage_like(
            "N\x08NA\x08AM\x08ME\x08E\n       tool - docs\n"
        ));
        assert!(!is_manpage_like("Commands:\n  run  Run a command\n"));
    }

    #[test]
    fn rejects_numeric_menu_entries_as_command_paths() {
        let text = "INTERACTIVE\n       1  status\n       2  update\n";
        let candidates = command_candidates(text, "git");

        assert!(candidates.is_empty());
    }

    #[test]
    fn extracts_simple_comma_separated_aliases_as_sibling_commands() {
        let text = "Commands:\n  rm, remove    Remove an item\n";
        let candidates = command_candidates(text, "tool");

        assert!(candidates.iter().any(|item| item.path == ["rm"]));
        assert!(candidates.iter().any(|item| item.path == ["remove"]));
        assert!(!candidates.iter().any(|item| item.path == ["rm", "remove"]));
        assert!(
            candidates
                .iter()
                .any(|item| item.path == ["rm"] && item.aliases == ["remove"])
        );
    }

    #[test]
    fn extracts_usage_positionals_from_current_command() {
        let text = "Usage: tool project deploy <PROJECT> [ENV] [FILES]...\n";
        let current_path = vec!["project".to_owned(), "deploy".to_owned()];
        let arguments = usage_arguments(text, "tool", &current_path);

        assert!(
            arguments
                .iter()
                .any(|arg| arg.name == "project" && arg.required)
        );
        assert!(
            arguments
                .iter()
                .any(|arg| arg.name == "env" && !arg.required)
        );
        assert!(
            arguments
                .iter()
                .any(|arg| arg.name == "files" && arg.variadic)
        );
    }

    #[test]
    fn usage_positionals_skip_flag_value_placeholders() {
        let text = "Usage: tool guard --baseline <FILE> <TARGET> [--format <KIND>]\n";
        let current_path = vec!["guard".to_owned()];
        let arguments = usage_arguments(text, "tool", &current_path);

        assert!(arguments.iter().any(|arg| arg.name == "target"));
        assert!(!arguments.iter().any(|arg| arg.name == "file"));
        assert!(!arguments.iter().any(|arg| arg.name == "kind"));
    }

    #[test]
    fn extracts_flag_value_kind_requiredness_and_repeatability() {
        let text = "Options:\n  -f, --format <KIND>       Output format\n  --color[=<WHEN>]          Optional color mode\n  --tag <TAG>...            Repeatable tag\n  --token <TOKEN>           Required authentication token\n  --dry-run                 Do not write changes\n";
        let flags = flag_candidates(text);

        let format = flags
            .iter()
            .find(|flag| flag.name == "--format")
            .expect("format flag");
        assert_eq!(format.short.as_deref(), Some("-f"));
        assert_eq!(format.value_kind, CandidateFlagValueKind::Required);
        assert_eq!(format.value_name.as_deref(), Some("kind"));

        let color = flags
            .iter()
            .find(|flag| flag.name == "--color")
            .expect("color flag");
        assert_eq!(color.value_kind, CandidateFlagValueKind::Optional);

        let tag = flags
            .iter()
            .find(|flag| flag.name == "--tag")
            .expect("tag flag");
        assert!(tag.repeatable);

        let token = flags
            .iter()
            .find(|flag| flag.name == "--token")
            .expect("token flag");
        assert!(token.required);

        let dry_run = flags
            .iter()
            .find(|flag| flag.name == "--dry-run")
            .expect("dry-run flag");
        assert_eq!(dry_run.value_kind, CandidateFlagValueKind::Boolean);
    }

    #[test]
    fn extracts_output_mode_candidates_from_structured_flags() {
        let text = "Options:\n  --json             Emit JSON\n  --format <KIND>    Output format: json or table\n  --output <FILE>    Output file\n";
        let candidates = output_mode_candidates(text);

        assert!(candidates.iter().any(|candidate| {
            candidate.mode == OutputMode::Json && candidate.argv_fragment == ["--json"]
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.mode == OutputMode::Json && candidate.argv_fragment == ["--format", "json"]
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.mode == OutputMode::Table && candidate.argv_fragment == ["--format", "table"]
        }));
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.flag_name == "--output")
        );
    }
}
