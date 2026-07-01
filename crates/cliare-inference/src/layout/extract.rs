use std::collections::BTreeMap;

use crate::layout_usage::{
    command_path_from_usage, usage_argument_from_token, usage_matches_current_path,
    usage_remainder_tokens, usage_syntaxes, value_name_from_token,
};

use super::commands::command_candidates;
use super::document::{ExtractionProfile, HelpDocument};
use super::flags::flag_candidates_from_document;
use super::output_modes::output_mode_candidates;
use super::types::CandidateArgument;

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
        .lines()
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
