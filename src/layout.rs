use std::collections::BTreeMap;

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

    for row in document.rows() {
        let Some(first_column) = row.columns.first() else {
            continue;
        };
        let Some(path) = command_path_from_cell(first_column, binary_name) else {
            continue;
        };

        candidates.entry(path.clone()).or_insert(CandidateCommand {
            path,
            summary: row.columns.get(1).cloned(),
            evidence_detail: format!("layout row {}", row.index),
        });
    }

    candidates.into_values().collect()
}

pub fn flag_candidates(text: &str) -> Vec<CandidateFlag> {
    let document = HelpDocument::parse(text);
    let mut candidates = BTreeMap::<String, CandidateFlag>::new();

    for row in document.rows() {
        for column in &row.columns {
            for token in column.split_whitespace() {
                let cleaned = clean_token(token);
                if !cleaned.starts_with('-') {
                    continue;
                }
                let name = if cleaned.starts_with("--") {
                    cleaned.to_owned()
                } else {
                    continue;
                };
                candidates.entry(name.clone()).or_insert(CandidateFlag {
                    name,
                    short: short_flag_in(column),
                    summary: row.columns.get(1).cloned(),
                    evidence_detail: format!("layout row {}", row.index),
                });
            }
        }
    }

    candidates.into_values().collect()
}

pub fn is_help_like(text: &str) -> bool {
    HelpDocument::parse(text).is_help_like()
}

#[derive(Debug, Clone)]
pub struct CandidateCommand {
    pub path: Vec<String>,
    pub summary: Option<String>,
    pub evidence_detail: String,
}

#[derive(Debug, Clone)]
pub struct CandidateFlag {
    pub name: String,
    pub short: Option<String>,
    pub summary: Option<String>,
    pub evidence_detail: String,
}

fn command_path_from_cell(cell: &str, binary_name: &str) -> Option<Vec<String>> {
    let mut path = Vec::new();

    for token in cell.split_whitespace() {
        let cleaned = clean_token(token);
        if cleaned.is_empty() {
            continue;
        }
        if cleaned == binary_name && path.is_empty() {
            continue;
        }
        if is_argument_like(cleaned) || cleaned.starts_with('-') {
            break;
        }
        if !is_command_token(cleaned) {
            break;
        }
        path.push(cleaned.to_owned());
    }

    (!path.is_empty()).then_some(path)
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
    use super::{command_candidates, flag_candidates, is_help_like};

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
}
