use std::path::PathBuf;

use clap::{ArgGroup, Args, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("report_filter")
        .args(["area", "issue"])
        .multiple(false)
))]
#[command(
    after_help = "For the end-to-end maintainer workflow and parameter guide, run: cliare playbook maintainer"
)]
pub struct ReportArgs {
    /// Persona packet to generate.
    #[arg(value_enum)]
    pub persona: ReportPersona,

    /// Measurement artifact directory containing scorecard.json, command-index.json, shape.json, and evidence.jsonl.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = ReportFormat::Markdown)]
    pub format: ReportFormat,

    /// Limit output to one agent-readiness area.
    #[arg(long, value_enum)]
    pub area: Option<ReportArea>,

    /// Limit output to one issue id, such as issue.output_mode_unprobed.
    #[arg(long, value_name = "ID")]
    pub issue: Option<String>,

    /// Include attached evidence entries in focused report output.
    #[arg(long)]
    pub with_evidence: bool,

    /// Write persona-<persona>.json and persona-<persona>.md into the artifact directory.
    #[arg(long)]
    pub write: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ReportPersona {
    Maintainer,
    Harness,
    Platform,
    Security,
    Oss,
    Devrel,
    Research,
}

impl ReportPersona {
    pub fn label(self) -> &'static str {
        match self {
            Self::Maintainer => "maintainer",
            Self::Harness => "harness",
            Self::Platform => "platform",
            Self::Security => "security",
            Self::Oss => "oss",
            Self::Devrel => "devrel",
            Self::Research => "research",
        }
    }
}

impl std::fmt::Display for ReportPersona {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    Markdown,
    Json,
    Bundle,
}

impl ReportFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Bundle => "bundle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ReportArea {
    OutputContracts,
    Preconditions,
    CommandDiscovery,
    HelpCoverage,
    Compatibility,
    Diagnostics,
    Execution,
    Safety,
    Coverage,
    Policy,
    Publishing,
    Calibration,
}

impl ReportArea {
    pub fn label(self) -> &'static str {
        match self {
            Self::OutputContracts => "output-contracts",
            Self::Preconditions => "preconditions",
            Self::CommandDiscovery => "command-discovery",
            Self::HelpCoverage => "help-coverage",
            Self::Compatibility => "compatibility",
            Self::Diagnostics => "diagnostics",
            Self::Execution => "execution",
            Self::Safety => "safety",
            Self::Coverage => "coverage",
            Self::Policy => "policy",
            Self::Publishing => "publishing",
            Self::Calibration => "calibration",
        }
    }
}

impl std::fmt::Display for ReportArea {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}
