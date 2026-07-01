use serde::Serialize;

use cliare_core::process_status::ProcessStatus;

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
    HelpText,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputHelpBehavior {
    MachineReadableHelp,
    HelpOverridesOutput,
    Ambiguous,
    PreconditionBlocked,
    ProcessFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutputHelpClassification {
    pub expected_mode: OutputMode,
    pub behavior: OutputHelpBehavior,
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
    if looks_like_help(text) {
        return OutputClassification {
            expected_mode,
            observed_kind: ObservedOutputKind::HelpText,
            parse_success: false,
            detail: "stdout was help text, not machine-readable command output".to_owned(),
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

pub fn classify_help_precedence(
    expected_mode: OutputMode,
    status: &ProcessStatus,
    stdout: Option<&str>,
) -> OutputHelpClassification {
    if !matches!(status, ProcessStatus::Exited { code: Some(0) }) {
        return OutputHelpClassification {
            expected_mode,
            behavior: OutputHelpBehavior::ProcessFailed,
            parse_success: false,
            detail: "output-help probe exited nonzero or did not finish".to_owned(),
        };
    }

    let text = stdout.unwrap_or("").trim();
    if text.is_empty() {
        return OutputHelpClassification {
            expected_mode,
            behavior: OutputHelpBehavior::Ambiguous,
            parse_success: false,
            detail: "output-help probe produced empty stdout".to_owned(),
        };
    }

    let direct = classify(expected_mode, status, Some(text));
    if direct.parse_success {
        return OutputHelpClassification {
            expected_mode,
            behavior: OutputHelpBehavior::MachineReadableHelp,
            parse_success: true,
            detail: format!(
                "--help respected {} output and produced machine-readable help",
                expected_mode.label()
            ),
        };
    }

    if looks_like_help(text) {
        return OutputHelpClassification {
            expected_mode,
            behavior: OutputHelpBehavior::HelpOverridesOutput,
            parse_success: false,
            detail: format!(
                "--help took precedence over {} output and produced prose help",
                expected_mode.label()
            ),
        };
    }

    OutputHelpClassification {
        expected_mode,
        behavior: OutputHelpBehavior::Ambiguous,
        parse_success: false,
        detail: format!(
            "output-help probe produced neither parseable {} nor recognizable help",
            expected_mode.label()
        ),
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

fn looks_like_help(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    lowercase.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("usage")
            || trimmed.starts_with("options")
            || trimmed.starts_with("flags")
            || trimmed.starts_with("commands")
            || trimmed.starts_with("subcommands")
            || trimmed.starts_with("arguments")
            || trimmed.contains(" --help")
            || trimmed.contains("[--help]")
    })
}

fn classify_yaml(expected_mode: OutputMode, text: &str) -> OutputClassification {
    if looks_like_script(text) {
        return OutputClassification {
            expected_mode,
            observed_kind: ObservedOutputKind::PlainText,
            parse_success: false,
            detail: "stdout looked like script text, not YAML".to_owned(),
        };
    }

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

fn looks_like_script(text: &str) -> bool {
    let sample = text.lines().take(40).collect::<Vec<_>>().join("\n");
    let lowercase = sample.to_ascii_lowercase();
    sample.starts_with("#!")
        || lowercase.contains("shell-script")
        || lowercase.contains("complete -")
        || lowercase.contains("function ")
        || sample.contains("()")
        || sample.contains("${")
        || sample.contains("[[")
        || sample.contains("fi\n")
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
