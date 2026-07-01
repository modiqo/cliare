use std::collections::BTreeMap;

use crate::output::OutputMode;

use super::document::{HelpDocument, LayoutLine};
use super::flags::flag_candidates_from_document;
use super::types::{
    CandidateFlag, CandidateFlagSection, CandidateFlagValueKind, CandidateOutputMode,
    CandidateOutputModeScope,
};

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

pub(super) fn example_line_mentions_output_mode(
    line: &str,
    flag: &CandidateFlag,
    mode: OutputMode,
) -> bool {
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

pub(super) fn is_json_fields_heading(text: &str) -> bool {
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

pub(super) fn json_fields_after(lines: &[LayoutLine]) -> Vec<String> {
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
