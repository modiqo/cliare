use std::collections::BTreeMap;

use crate::layout_tokens::{clean_token, long_flag_name, short_flag_in};
use crate::layout_usage::{bare_value_name_from_token, value_name_from_token};

use super::document::HelpDocument;
use super::types::{CandidateFlag, CandidateFlagValueKind};

pub fn flag_candidates(text: &str) -> Vec<CandidateFlag> {
    let document = HelpDocument::parse(text);
    flag_candidates_from_document(&document)
}

pub(super) fn flag_candidates_from_document(document: &HelpDocument) -> Vec<CandidateFlag> {
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
