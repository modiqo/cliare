use crate::layout_tokens::split_columns;
use crate::output::OutputMode;

use super::output_modes::{
    example_line_mentions_output_mode, is_json_fields_heading, json_fields_after,
};
use super::sections::{SectionKind, flag_section, is_example_section, section_kind};
use super::types::{CandidateFlag, CandidateFlagSection};

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

    pub(super) fn section_kind_before(&self, row_index: usize) -> SectionKind {
        self.section_title_before(row_index)
            .map_or(SectionKind::Unknown, section_kind)
    }

    pub(super) fn section_title_before(&self, row_index: usize) -> Option<&str> {
        self.lines[..row_index]
            .iter()
            .rev()
            .find(|line| !line.text.is_empty() && !line.is_row_like())
            .map(|line| line.text.as_str())
    }

    pub(super) fn flag_section_before(&self, row_index: usize) -> CandidateFlagSection {
        self.section_title_before(row_index)
            .map_or(CandidateFlagSection::Other, flag_section)
    }

    pub(super) fn examples_advertise_output_mode(
        &self,
        flag: &CandidateFlag,
        mode: OutputMode,
    ) -> bool {
        self.lines.iter().any(|line| {
            self.section_title_before(line.index)
                .is_some_and(is_example_section)
                && example_line_mentions_output_mode(&line.text, flag, mode)
        })
    }

    pub(super) fn json_field_probe_value(&self) -> Option<String> {
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

    pub(super) fn row_count(&self) -> usize {
        self.rows().count()
    }

    pub(super) fn header_count(&self) -> usize {
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

    pub(super) fn is_row_like(&self) -> bool {
        self.indent > 0 && self.columns.len() >= 2 && !self.text.is_empty()
    }

    fn is_continuation_like(&self) -> bool {
        self.columns
            .first()
            .is_some_and(|column| column.starts_with("--") && self.indent > 6)
    }
}
