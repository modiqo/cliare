use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum, ValueHint};

#[derive(Debug, Args)]
pub struct SkillsArgs {
    #[command(subcommand)]
    pub command: SkillsCommand,
}

#[derive(Debug, Subcommand)]
pub enum SkillsCommand {
    /// List installable CLIARE agent skill targets.
    List(SkillsListArgs),
    /// Install CLIARE skills into a local agent configuration directory.
    Install(SkillsInstallArgs),
}

#[derive(Debug, Args)]
pub struct SkillsListArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = SkillsListFormat::Text)]
    pub format: SkillsListFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SkillsListFormat {
    Text,
    Json,
}

#[derive(Debug, Args)]
pub struct SkillsInstallArgs {
    /// Agent integration to install.
    #[arg(long, value_enum, default_value_t = SkillAgent::All)]
    pub agent: SkillAgent,

    /// Install into user-level or project-level agent directories.
    #[arg(long, value_enum, default_value_t = SkillInstallScope::User)]
    pub scope: SkillInstallScope,

    /// User home directory override for user-scope installs.
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub home: Option<PathBuf>,

    /// Project directory override for project-scope installs.
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub project_dir: Option<PathBuf>,

    /// Show planned writes without changing files.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SkillAgent {
    All,
    Claude,
    Codex,
    Cursor,
}

impl SkillAgent {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
        }
    }
}

impl std::fmt::Display for SkillAgent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SkillInstallScope {
    User,
    Project,
}

impl SkillInstallScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

impl std::fmt::Display for SkillInstallScope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}
