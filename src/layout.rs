use std::collections::BTreeMap;

use crate::layout_tokens::{
    clean_token, is_argument_like, is_command_token, is_placeholder_like, long_flag_name,
    short_flag_in, split_columns, starts_title_like,
};
use crate::layout_usage::{
    bare_value_name_from_token, command_path_from_usage, usage_argument_from_token,
    usage_matches_current_path, usage_remainder_tokens, usage_syntaxes, value_name_from_token,
};
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

    pub(crate) fn lines(&self) -> &[LayoutLine] {
        &self.lines
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

    fn section_kind_before(&self, row_index: usize) -> SectionKind {
        self.section_title_before(row_index)
            .map_or(SectionKind::Unknown, section_kind)
    }

    fn section_title_before(&self, row_index: usize) -> Option<&str> {
        self.lines[..row_index]
            .iter()
            .rev()
            .find(|line| !line.text.is_empty() && !line.is_row_like())
            .map(|line| line.text.as_str())
    }

    fn flag_section_before(&self, row_index: usize) -> CandidateFlagSection {
        self.section_title_before(row_index)
            .map_or(CandidateFlagSection::Other, flag_section)
    }

    fn examples_advertise_output_mode(&self, flag: &CandidateFlag, mode: OutputMode) -> bool {
        self.lines.iter().any(|line| {
            self.section_title_before(line.index)
                .is_some_and(is_example_section)
                && example_line_mentions_output_mode(&line.text, flag, mode)
        })
    }

    fn json_field_probe_value(&self) -> Option<String> {
        let fields = self
            .lines
            .iter()
            .enumerate()
            .find(|(_, line)| is_json_fields_heading(&line.text))
            .map(|(index, _)| json_fields_after(&self.lines[index + 1..]))
            .unwrap_or_default();
        if fields.is_empty() {
            None
        } else {
            Some(fields.into_iter().take(3).collect::<Vec<_>>().join(","))
        }
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
        let header_count = self.header_count();
        let option_count = self
            .lines
            .iter()
            .filter(|line| line.tokens.iter().any(|token| token.starts_with('-')))
            .count();

        row_count >= 2 || (header_count >= 1 && row_count >= 1) || option_count >= 1
    }

    fn row_count(&self) -> usize {
        self.rows().count()
    }

    fn header_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| line.is_header_like())
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractionProfile {
    pub help_like: bool,
    pub manpage_like: bool,
    pub row_count: usize,
    pub header_count: usize,
    pub command_candidates: usize,
    pub flag_candidates: usize,
    pub output_mode_candidates: usize,
    pub usage_arguments: usize,
}

impl ExtractionProfile {
    pub fn shape_signal_count(&self) -> usize {
        self.command_candidates
            + self.flag_candidates
            + self.output_mode_candidates
            + self.usage_arguments
    }

    pub fn has_shape_signal(&self) -> bool {
        self.shape_signal_count() > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionKind {
    Command,
    NonCommand,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateFlagSection {
    Command,
    Global,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateOutputModeScope {
    CommandFlag,
    GlobalFlag,
    Example,
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

    pub(crate) fn is_header_like(&self) -> bool {
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
        let section = document.section_kind_before(block[0].index);
        if section == SectionKind::NonCommand {
            continue;
        }
        let rows = block
            .iter()
            .filter_map(|row| command_row_candidate(row, binary_name))
            .collect::<Vec<_>>();
        if !is_command_table_block(block.len(), &rows, section) {
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
                    absolute: row.absolute,
                    evidence_detail: format!("layout row {}", row.index),
                });
            }
        }
    }

    candidates.into_values().collect()
}

pub fn flag_candidates(text: &str) -> Vec<CandidateFlag> {
    let document = HelpDocument::parse(text);
    flag_candidates_from_document(&document)
}

fn flag_candidates_from_document(document: &HelpDocument) -> Vec<CandidateFlag> {
    let mut candidates = BTreeMap::<String, CandidateFlag>::new();

    for row in document.rows() {
        let section = document.flag_section_before(row.index);
        for column in &row.columns {
            for token in column.split_whitespace() {
                let Some(name) = long_flag_name(token) else {
                    continue;
                };
                let grammar = flag_grammar_in(column, row.columns.get(1).map(String::as_str));
                candidates.entry(name.clone()).or_insert(CandidateFlag {
                    name,
                    short: short_flag_in(column),
                    invocation: column.clone(),
                    summary: row.columns.get(1).cloned(),
                    section,
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
    let document = HelpDocument::parse(text);
    let json_field_probe_value = document.json_field_probe_value();
    let mut candidates = BTreeMap::<(OutputMode, Vec<String>), CandidateOutputMode>::new();

    for flag in flag_candidates_from_document(&document) {
        for mode in output_modes_for_flag(&flag) {
            let argv_fragment =
                output_mode_argv_fragment(&flag, mode, json_field_probe_value.as_deref());
            let scope = output_mode_scope(&document, &flag, mode);
            candidates
                .entry((mode, argv_fragment.clone()))
                .or_insert_with(|| CandidateOutputMode {
                    mode,
                    flag_name: flag.name.clone(),
                    argv_fragment,
                    scope,
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

    for (line_index, usage) in usage_syntaxes(&document) {
        if !usage_matches_current_path(&usage, binary_name, current_path) {
            continue;
        }

        let tokens = usage_remainder_tokens(&usage, binary_name, current_path);
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
            if let Some(argument) = usage_argument_from_token(token, line_index) {
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

pub fn extraction_profile(
    text: &str,
    binary_name: &str,
    current_path: &[String],
) -> ExtractionProfile {
    let document = HelpDocument::parse(text);
    let manpage_like = is_manpage_like(text);
    let command_candidates = if current_path.is_empty() || !manpage_like {
        command_candidates(text, binary_name).len()
    } else {
        0
    };
    let usage_scope = usage_command_path(text, binary_name, current_path).unwrap_or_default();

    ExtractionProfile {
        help_like: document.is_help_like(),
        manpage_like,
        row_count: document.row_count(),
        header_count: document.header_count(),
        command_candidates,
        flag_candidates: flag_candidates_from_document(&document).len(),
        output_mode_candidates: output_mode_candidates(text).len(),
        usage_arguments: usage_arguments(text, binary_name, &usage_scope).len(),
    }
}

pub fn help_matches_command_path(text: &str, binary_name: &str, current_path: &[String]) -> bool {
    if current_path.is_empty() {
        return true;
    }

    let document = HelpDocument::parse(text);
    let syntaxes = usage_syntaxes(&document);
    if !syntaxes.is_empty() {
        return syntaxes
            .iter()
            .any(|(_, usage)| usage_matches_current_path(usage, binary_name, current_path));
    }

    let expected = std::iter::once(binary_name)
        .chain(current_path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    document
        .lines
        .iter()
        .find(|line| !line.text.is_empty())
        .is_some_and(|line| {
            line.text == expected
                || line
                    .text
                    .strip_prefix(&expected)
                    .is_some_and(|rest| rest.starts_with(' ') || rest.starts_with(" -"))
        })
}

pub fn usage_command_path(
    text: &str,
    binary_name: &str,
    current_path: &[String],
) -> Option<Vec<String>> {
    let document = HelpDocument::parse(text);
    usage_syntaxes(&document)
        .into_iter()
        .filter_map(|(_, usage)| command_path_from_usage(&usage, binary_name, current_path))
        .max_by_key(Vec::len)
}

pub fn is_manpage_like(text: &str) -> bool {
    text.as_bytes().windows(2).any(|window| window[1] == 0x08)
}

#[derive(Debug, Clone)]
pub struct CandidateCommand {
    pub path: Vec<String>,
    pub aliases: Vec<String>,
    pub summary: Option<String>,
    pub absolute: bool,
    pub evidence_detail: String,
}

#[derive(Debug, Clone)]
pub struct CandidateFlag {
    pub name: String,
    pub short: Option<String>,
    pub invocation: String,
    pub summary: Option<String>,
    pub section: CandidateFlagSection,
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
    pub scope: CandidateOutputModeScope,
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
    absolute: bool,
    placeholder_like: bool,
    invocation_width: usize,
    invocation_tokens: usize,
    title_like_summary: bool,
    distinctive_invocation: bool,
}

fn command_row_candidate(row: &LayoutLine, binary_name: &str) -> Option<CommandRowCandidate> {
    let first_column = row.columns.first()?;
    let parsed = command_paths_from_cell(first_column, binary_name);
    if parsed.paths.is_empty() {
        return None;
    }
    let distinctive_invocation = parsed.paths.iter().flatten().any(|segment| {
        segment
            .chars()
            .any(|ch| !ch.is_ascii_lowercase() || matches!(ch, '-' | '_' | ':' | '@'))
    });
    Some(CommandRowCandidate {
        index: row.index,
        placeholder_like: parsed.paths.iter().all(|path| {
            path.iter()
                .all(|segment| is_placeholder_like(segment.as_str()))
        }),
        paths: parsed.paths,
        summary: row.columns.get(1).cloned(),
        absolute: parsed.absolute,
        invocation_width: first_column.chars().count(),
        invocation_tokens: command_prefix_token_count(first_column, binary_name),
        title_like_summary: row
            .columns
            .get(1)
            .is_some_and(|summary| starts_title_like(summary)),
        distinctive_invocation,
    })
}

fn is_command_table_block(
    row_count: usize,
    candidates: &[CommandRowCandidate],
    section: SectionKind,
) -> bool {
    if candidates.len() == 1 {
        let row = &candidates[0];
        return row.invocation_width <= 48
            && row.invocation_tokens <= 3
            && (!row.placeholder_like || section == SectionKind::Command)
            && (row.paths.len() > 1 || row.title_like_summary || row.distinctive_invocation);
    }
    if candidates.len() < 2 {
        return false;
    }
    if section != SectionKind::Command && candidates.iter().all(|row| row.placeholder_like) {
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

fn section_kind(text: &str) -> SectionKind {
    let normalized = text
        .trim_matches(|ch: char| matches!(ch, ':' | '-' | '='))
        .to_ascii_lowercase();
    let words = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    if words
        .iter()
        .any(|word| matches!(*word, "commands" | "command" | "subcommands" | "subcommand"))
    {
        return SectionKind::Command;
    }

    if words.iter().any(|word| {
        matches!(
            *word,
            "arguments"
                | "argument"
                | "parameters"
                | "parameter"
                | "options"
                | "option"
                | "flags"
                | "flag"
                | "examples"
                | "example"
                | "usage"
                | "environment"
                | "outputs"
                | "output"
                | "inputs"
                | "input"
                | "keys"
                | "key"
                | "values"
                | "value"
                | "fields"
                | "field"
                | "types"
                | "type"
                | "exit"
                | "codes"
                | "code"
                | "notes"
                | "note"
                | "details"
                | "location"
                | "locations"
                | "provider"
                | "providers"
        )
    }) {
        return SectionKind::NonCommand;
    }

    SectionKind::Unknown
}

fn flag_section(text: &str) -> CandidateFlagSection {
    let normalized = text
        .trim_matches(|ch: char| matches!(ch, ':' | '-' | '='))
        .to_ascii_lowercase();
    let words = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let flag_like = words.iter().any(|word| {
        matches!(
            *word,
            "flags" | "flag" | "options" | "option" | "switches" | "switch"
        )
    });

    if !flag_like {
        return CandidateFlagSection::Other;
    }
    if words
        .iter()
        .any(|word| matches!(*word, "global" | "persistent" | "inherited" | "common"))
    {
        CandidateFlagSection::Global
    } else {
        CandidateFlagSection::Command
    }
}

fn is_example_section(text: &str) -> bool {
    let normalized = text
        .trim_matches(|ch: char| matches!(ch, ':' | '-' | '='))
        .to_ascii_lowercase();
    normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| matches!(word, "example" | "examples"))
}

#[derive(Debug, Default)]
struct ParsedCommandPaths {
    paths: Vec<Vec<String>>,
    absolute: bool,
}

fn command_paths_from_cell(cell: &str, binary_name: &str) -> ParsedCommandPaths {
    let mut path = Vec::new();
    let mut absolute = false;
    let mut saw_alias_separator = false;

    for token in cell.split_whitespace() {
        let cleaned = clean_token(token);
        if cleaned.is_empty() {
            continue;
        }
        if cleaned == binary_name && path.is_empty() {
            absolute = true;
            continue;
        }
        if cleaned.chars().all(|ch| ch.is_ascii_digit()) {
            return ParsedCommandPaths::default();
        }
        if is_argument_like(cleaned) || cleaned.starts_with('-') {
            break;
        }
        if !path.is_empty() && is_placeholder_like(cleaned) {
            break;
        }
        if !is_command_token(cleaned) {
            break;
        }
        if path.len() >= MAX_COMMAND_PATH_SEGMENTS {
            return ParsedCommandPaths::default();
        }
        saw_alias_separator |= token.ends_with(',');
        path.push(cleaned.to_owned());
    }

    if path.is_empty() {
        return ParsedCommandPaths::default();
    }
    if saw_alias_separator {
        return ParsedCommandPaths {
            paths: path.into_iter().map(|segment| vec![segment]).collect(),
            absolute,
        };
    }

    ParsedCommandPaths {
        paths: vec![path],
        absolute,
    }
}

fn command_prefix_token_count(cell: &str, binary_name: &str) -> usize {
    command_paths_from_cell(cell, binary_name)
        .paths
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
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
            if next.starts_with("[<") || *next == "[" {
                value_kind = CandidateFlagValueKind::Optional;
                value_name = value_name_from_token(next).or_else(|| Some("choice".to_owned()));
                repeatable |= next.contains("...");
            } else if next.starts_with('<') {
                value_kind = CandidateFlagValueKind::Required;
                value_name = value_name_from_token(next);
                repeatable |= next.contains("...");
            } else if let Some(name) = bare_value_name_from_token(next) {
                value_kind = CandidateFlagValueKind::Required;
                value_name = Some(name);
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

fn output_modes_for_flag(flag: &CandidateFlag) -> Vec<OutputMode> {
    let mut modes = Vec::new();

    for (needle, mode) in [
        ("json", OutputMode::Json),
        ("yaml", OutputMode::Yaml),
        ("yml", OutputMode::Yaml),
        ("table", OutputMode::Table),
        ("plain", OutputMode::Plain),
    ] {
        if flag_advertises_output_mode(flag, needle) && !modes.contains(&mode) {
            modes.push(mode);
        }
    }

    modes
}

fn output_mode_scope(
    document: &HelpDocument,
    flag: &CandidateFlag,
    mode: OutputMode,
) -> CandidateOutputModeScope {
    if document.examples_advertise_output_mode(flag, mode) {
        CandidateOutputModeScope::Example
    } else if flag.section == CandidateFlagSection::Global {
        CandidateOutputModeScope::GlobalFlag
    } else {
        CandidateOutputModeScope::CommandFlag
    }
}

fn example_line_mentions_output_mode(line: &str, flag: &CandidateFlag, mode: OutputMode) -> bool {
    let normalized = line.to_ascii_lowercase();
    contains_word(&normalized, mode.label())
        && (flag_reference_in_line(&normalized, &flag.name)
            || flag
                .short
                .as_deref()
                .is_some_and(|short| flag_reference_in_line(&normalized, short)))
}

fn flag_reference_in_line(line: &str, flag: &str) -> bool {
    if flag.is_empty() {
        return false;
    }
    let flag = flag.to_ascii_lowercase();
    line.split_whitespace().any(|token| {
        token == flag
            || token
                .strip_prefix(&flag)
                .is_some_and(|suffix| suffix.starts_with('=') || suffix.starts_with(','))
    })
}

fn flag_advertises_output_mode(flag: &CandidateFlag, mode_word: &str) -> bool {
    let flag_name = flag.name.trim_start_matches("--").replace(['-', '_'], " ");
    let value_name = flag.value_name.as_deref().unwrap_or("").replace('_', " ");
    let summary = flag.summary.as_deref().unwrap_or("").to_ascii_lowercase();

    if summary_describes_output_modifier(&summary, mode_word)
        && !flag_name_describes_output_selector(&flag_name, &value_name)
    {
        return false;
    }

    if flag.value_kind == CandidateFlagValueKind::Boolean
        && contains_word(&flag_name, mode_word)
        && (mode_flag_name_is_direct_output(&flag_name, mode_word)
            || summary_describes_output_mode(&summary, mode_word))
    {
        return true;
    }

    if !contains_word(&summary, mode_word) {
        return flag_invocation_advertises_output_choice(flag, &flag_name, &summary, mode_word);
    }

    flag_name_describes_output_selector(&flag_name, &value_name)
        || summary.contains("output format")
        || summary.contains("format:")
        || summary.contains("format as")
        || summary_describes_output_mode(&summary, mode_word)
        || summary.contains("machine-readable")
}

fn flag_name_describes_output_selector(flag_name: &str, value_name: &str) -> bool {
    contains_word(flag_name, "format")
        || contains_word(value_name, "format")
        || contains_word(value_name, "mode")
        || contains_word(value_name, "kind")
        || contains_word(value_name, "fields")
}

fn summary_describes_output_modifier(summary: &str, mode_word: &str) -> bool {
    (summary.contains(&format!("filter {mode_word} output"))
        || summary.contains(&format!("filtering {mode_word} output"))
        || summary.contains(&format!("format {mode_word} output"))
        || summary.contains(&format!("formatting {mode_word} output")))
        && (summary.contains(" using ")
            || summary.contains(" expression")
            || summary.contains(" template"))
}

fn flag_invocation_advertises_output_choice(
    flag: &CandidateFlag,
    flag_name: &str,
    summary: &str,
    mode_word: &str,
) -> bool {
    if !contains_word(&flag.invocation.to_ascii_lowercase(), mode_word) {
        return false;
    }
    if !(contains_word(flag_name, "output")
        || contains_word(flag_name, "format")
        || contains_word(summary, "output")
        || contains_word(summary, "format"))
    {
        return false;
    }
    flag.invocation.contains('|') || flag.invocation.contains('[') || flag.invocation.contains('<')
}

fn mode_flag_name_is_direct_output(flag_name: &str, mode_word: &str) -> bool {
    let words = flag_name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    words == [mode_word]
        || words == ["output", mode_word]
        || words == [mode_word, "output"]
        || words == ["format", mode_word]
        || words == [mode_word, "format"]
}

fn summary_describes_output_mode(summary: &str, mode_word: &str) -> bool {
    [
        "emit", "emits", "print", "prints", "output", "outputs", "render", "renders", "produce",
        "produces", "write", "writes",
    ]
    .into_iter()
    .any(|verb| summary.contains(&format!("{verb} {mode_word}")))
        || summary.contains(&format!("result as {mode_word}"))
        || summary.contains(&format!("output as {mode_word}"))
        || summary.contains(&format!("format as {mode_word}"))
        || summary.contains(&format!("{mode_word} output"))
}

fn output_mode_argv_fragment(
    flag: &CandidateFlag,
    mode: OutputMode,
    json_field_probe_value: Option<&str>,
) -> Vec<String> {
    if flag.value_kind == CandidateFlagValueKind::Boolean {
        vec![flag.name.clone()]
    } else if mode == OutputMode::Json
        && flag
            .value_name
            .as_deref()
            .is_some_and(is_json_fields_value_name)
        && let Some(value) = json_field_probe_value
    {
        vec![flag.name.clone(), value.to_owned()]
    } else {
        vec![flag.name.clone(), mode.label().to_owned()]
    }
}

fn is_json_fields_heading(text: &str) -> bool {
    let words = text
        .trim_matches(|ch: char| matches!(ch, ':' | '-' | '='))
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    words.len() <= 3
        && words.iter().any(|word| word == "json")
        && words.iter().any(|word| word == "fields" || word == "field")
}

fn json_fields_after(lines: &[LayoutLine]) -> Vec<String> {
    let mut fields = Vec::new();
    for line in lines {
        if line.text.is_empty() {
            if fields.is_empty() {
                continue;
            }
            break;
        }
        if !fields.is_empty() && !line.is_row_like() && line.is_header_like() {
            break;
        }
        for field in json_fields_from_line(&line.text) {
            if !fields.contains(&field) {
                fields.push(field);
            }
        }
        if fields.len() >= 3 {
            break;
        }
    }
    fields
}

fn json_fields_from_line(line: &str) -> Vec<String> {
    line.split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter_map(json_field_token)
        .collect()
}

fn json_field_token(token: &str) -> Option<String> {
    let cleaned = token.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ':' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\''
        )
    });
    if cleaned.is_empty()
        || cleaned.starts_with('-')
        || cleaned.eq_ignore_ascii_case("and")
        || cleaned.eq_ignore_ascii_case("or")
    {
        return None;
    }
    let mut chars = cleaned.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')) {
        return None;
    }
    Some(cleaned.to_owned())
}

fn is_json_fields_value_name(value_name: &str) -> bool {
    matches!(value_name, "field" | "fields" | "json_fields")
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

#[cfg(test)]
mod tests {
    use super::{
        CandidateFlagValueKind, command_candidates, flag_candidates, help_matches_command_path,
        is_help_like, is_manpage_like, output_mode_candidates, usage_arguments, usage_command_path,
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
    fn rejects_argument_tables_as_command_candidates() {
        let text = "ARGUMENTS\n  FILE  Path to archive file\n  NAME  Resource name\n";
        let candidates = command_candidates(text, "tool");

        assert!(candidates.is_empty());
    }

    #[test]
    fn rejects_key_value_tables_as_command_candidates() {
        let text = "SETTABLE KEYS\n  base_url     url     http(s) required\n  tags         csv     comma-separated labels\n";
        let candidates = command_candidates(text, "tool");

        assert!(candidates.is_empty());
    }

    #[test]
    fn keeps_uppercase_commands_in_command_sections() {
        let text = "COMMANDS\n  GET  Execute HTTP GET request\n";
        let candidates = command_candidates(text, "tool");

        assert!(candidates.iter().any(|item| item.path == ["GET"]));
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
    fn absolute_invocation_rows_keep_root_paths_and_skip_placeholders() {
        let text = "NEXT STEPS\n  rote adapter info <ID>      Show details\n  rote adapter new ID SPEC    Create an adapter\n";
        let candidates = command_candidates(text, "rote");

        assert!(
            candidates
                .iter()
                .any(|item| { item.path == ["adapter", "info"] && item.absolute })
        );
        assert!(
            candidates
                .iter()
                .any(|item| { item.path == ["adapter", "new"] && item.absolute })
        );
        assert!(
            !candidates
                .iter()
                .any(|item| item.path == ["adapter", "new", "ID"])
        );
        assert!(
            !candidates
                .iter()
                .any(|item| item.path == ["adapter", "new", "ID", "SPEC"])
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
    fn extracts_usage_positionals_from_header_blocks_and_skips_sibling_usage() {
        let text = "USAGE\n  tool adapter new <ID> <SPEC> [OPTIONS]\n  tool adapter new-from-mcp <ID> <MCP_ENDPOINT>\n\nARGUMENTS\n  ID  Identifier\n";
        let current_path = vec!["adapter".to_owned(), "new".to_owned()];
        let arguments = usage_arguments(text, "tool", &current_path);

        assert!(arguments.iter().any(|arg| arg.name == "id" && arg.required));
        assert!(
            arguments
                .iter()
                .any(|arg| arg.name == "spec" && arg.required)
        );
        assert!(!arguments.iter().any(|arg| arg.name == "mcp_endpoint"));
    }

    #[test]
    fn detects_when_help_usage_matches_the_probed_command_path() {
        let text = "tool adapter set - Mutate a key\n\nUSAGE\n  tool adapter set <ID> <KEY> <VALUE> [--json]\n";

        assert!(help_matches_command_path(
            text,
            "tool",
            &["adapter".to_owned(), "set".to_owned()]
        ));
        assert!(!help_matches_command_path(
            text,
            "tool",
            &[
                "adapter".to_owned(),
                "set".to_owned(),
                "base_url".to_owned()
            ]
        ));
    }

    #[test]
    fn detects_multiline_usage_as_matching_command_path() {
        let text = "Manage Supabase physical backups\n\nUsage:\n  supabase backups [command]\n\nAvailable Commands:\n  list     Lists available physical backups\n  restore  Restore to a specific timestamp using PITR\n";

        assert!(help_matches_command_path(
            text,
            "supabase",
            &["backups".to_owned()]
        ));
        assert_eq!(
            usage_command_path(text, "supabase", &["backups".to_owned()]),
            Some(vec!["backups".to_owned()])
        );
    }

    #[test]
    fn extracts_usage_command_scope_for_parent_help_echoes() {
        let text = "USAGE\n  tool adapter set <ID> <KEY> <VALUE> [--json]\n";
        let current_path = vec![
            "adapter".to_owned(),
            "set".to_owned(),
            "base_url".to_owned(),
        ];

        assert_eq!(
            usage_command_path(text, "tool", &current_path),
            Some(vec!["adapter".to_owned(), "set".to_owned()])
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

    #[test]
    fn extracts_output_modes_from_choice_lists_in_flag_invocation() {
        let text = "Flags:\n  -o, --output [ env | pretty | json | toml | yaml ]   output format of status variables (default pretty)\n";
        let candidates = output_mode_candidates(text);

        assert!(candidates.iter().any(|candidate| {
            candidate.mode == OutputMode::Json
                && candidate.flag_name == "--output"
                && candidate.argv_fragment == ["--output", "json"]
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.mode == OutputMode::Yaml
                && candidate.flag_name == "--output"
                && candidate.argv_fragment == ["--output", "yaml"]
        }));
    }

    #[test]
    fn extracts_json_field_selector_probe_values() {
        let text = "FLAGS\n  -q, --jq expression      Filter JSON output using a jq expression\n      --json fields        Output JSON with the specified fields\n  -t, --template string    Format JSON output using a Go template\n\nJSON FIELDS\n  assignees, author, body, closed, closedAt, comments, createdAt, id,\n  labels, milestone, number, state, title, updatedAt, url\n";
        let candidates = output_mode_candidates(text);

        assert!(candidates.iter().any(|candidate| {
            candidate.mode == OutputMode::Json
                && candidate.flag_name == "--json"
                && candidate.argv_fragment == ["--json", "assignees,author,body"]
        }));
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.flag_name == "--jq")
        );
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.flag_name == "--template")
        );
    }

    #[test]
    fn ignores_json_file_defaults_as_output_modes() {
        let text = "Options:\n  --manifest <FILE>  Benchmark corpus manifest [default: benchmarks/local-corpus.json]\n  --output <FILE>    Output file [default: report.json]\n";
        let candidates = output_mode_candidates(text);

        assert!(candidates.is_empty());
    }

    #[test]
    fn ignores_json_input_payload_flags_as_output_modes() {
        let text = "Options:\n  --config <FILE>       Load configuration from JSON file\n  --config-json <JSON>  Pass auth/headers/filters as JSON\n  --dry-run             Analyze without creating and output JSON\n";
        let candidates = output_mode_candidates(text);

        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.flag_name == "--config")
        );
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.flag_name == "--config-json")
        );
        assert!(candidates.iter().any(|candidate| {
            candidate.mode == OutputMode::Json && candidate.argv_fragment == ["--dry-run"]
        }));
    }

    #[test]
    fn ignores_help_text_that_mentions_another_output_flag() {
        let text = "Options:\n  --format <FORMAT>  Output format [default: text] [possible values: text, json]\n  --help             Print help. With --format json, emit a parseable metadata contract\n";
        let candidates = output_mode_candidates(text);

        assert!(candidates.iter().any(|candidate| {
            candidate.mode == OutputMode::Json && candidate.argv_fragment == ["--format", "json"]
        }));
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.flag_name == "--help")
        );
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.argv_fragment == ["--format", "plain"])
        );
    }
}
