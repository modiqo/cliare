use serde::Serialize;

use crate::evidence::ProcessStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Json,
    Yaml,
    Table,
    Plain,
}

impl OutputMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Table => "table",
            Self::Plain => "plain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedOutputKind {
    Json,
    YamlLike,
    TableLike,
    PlainText,
    Empty,
    Unparseable,
    ProcessFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutputClassification {
    pub expected_mode: OutputMode,
    pub observed_kind: ObservedOutputKind,
    pub parse_success: bool,
    pub detail: String,
}

pub fn classify(
    expected_mode: OutputMode,
    status: &ProcessStatus,
    stdout: Option<&str>,
) -> OutputClassification {
    if !matches!(status, ProcessStatus::Exited { code: Some(0) }) {
        return OutputClassification {
            expected_mode,
            observed_kind: ObservedOutputKind::ProcessFailed,
            parse_success: false,
            detail: "probe exited nonzero or did not finish".to_owned(),
        };
    }

    let text = stdout.unwrap_or("").trim();
    if text.is_empty() {
        return OutputClassification {
            expected_mode,
            observed_kind: ObservedOutputKind::Empty,
            parse_success: false,
            detail: "stdout was empty".to_owned(),
        };
    }

    match expected_mode {
        OutputMode::Json => classify_json(expected_mode, text),
        OutputMode::Yaml => classify_yaml(expected_mode, text),
        OutputMode::Table => classify_table(expected_mode, text),
        OutputMode::Plain => OutputClassification {
            expected_mode,
            observed_kind: ObservedOutputKind::PlainText,
            parse_success: true,
            detail: "plain text was produced".to_owned(),
        },
    }
}

fn classify_json(expected_mode: OutputMode, text: &str) -> OutputClassification {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(_) => OutputClassification {
            expected_mode,
            observed_kind: ObservedOutputKind::Json,
            parse_success: true,
            detail: "stdout parsed as JSON".to_owned(),
        },
        Err(source) => OutputClassification {
            expected_mode,
            observed_kind: ObservedOutputKind::Unparseable,
            parse_success: false,
            detail: format!("stdout did not parse as JSON: {source}"),
        },
    }
}

fn classify_yaml(expected_mode: OutputMode, text: &str) -> OutputClassification {
    let yaml_like = text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("- ")
            || trimmed
                .split_once(':')
                .is_some_and(|(key, value)| !key.trim().is_empty() && !value.trim().is_empty())
    });

    OutputClassification {
        expected_mode,
        observed_kind: if yaml_like {
            ObservedOutputKind::YamlLike
        } else {
            ObservedOutputKind::Unparseable
        },
        parse_success: yaml_like,
        detail: if yaml_like {
            "stdout matched conservative YAML-like structure".to_owned()
        } else {
            "stdout did not match conservative YAML-like structure".to_owned()
        },
    }
}

fn classify_table(expected_mode: OutputMode, text: &str) -> OutputClassification {
    let rows = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.contains('|') || trimmed.split_whitespace().count() >= 2
        })
        .count();

    OutputClassification {
        expected_mode,
        observed_kind: if rows >= 2 {
            ObservedOutputKind::TableLike
        } else {
            ObservedOutputKind::PlainText
        },
        parse_success: rows >= 2,
        detail: if rows >= 2 {
            "stdout matched table-like rows".to_owned()
        } else {
            "stdout did not contain enough table-like rows".to_owned()
        },
    }
}
