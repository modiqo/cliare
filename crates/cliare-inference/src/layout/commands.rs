use std::collections::BTreeMap;

use crate::layout_tokens::{
    clean_token, is_argument_like, is_command_token, is_placeholder_like, starts_title_like,
};

use super::MAX_COMMAND_PATH_SEGMENTS;
use super::document::{HelpDocument, LayoutLine};
use super::sections::SectionKind;
use super::types::CandidateCommand;

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
