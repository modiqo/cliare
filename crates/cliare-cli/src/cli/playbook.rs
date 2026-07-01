use std::path::PathBuf;

use clap::{Args, ValueEnum, ValueHint};

#[derive(Debug, Args)]
#[command(
    long_about = "Print a role-specific operational playbook. Maintainers get the measure, view, act, disposition, remeasure, CI, and agent-surface publishing loop. Harness and security teams get focused execution loops over the same CLIARE artifacts.",
    after_help = "Available playbooks:
  maintainer  Measure, inspect, fix or disposition, remeasure, gate in CI, and publish the agent surface.
  harness     Consume the command index, harness packet, and generated skill to route agents through the CLI deliberately.
  security    Review safety, credential-like side effects, host/auth exposure, and policy evidence before approving agent use.

Maintainer workflow:
  1. Measure: cliare measure <target-cli> --out .cliare/<target-cli> --profile quick|standard|deep --refresh
  2. View: cliare report maintainer --out .cliare/<target-cli> --format markdown
  3. Act or disposition: fix the CLI, or use cliare issues mark <issue-id> --status intentional|needs-fixture
  4. Remeasure: cliare measure <target-cli> --out .cliare/<target-cli> --profile deep --refresh
  5. Gate: cliare guard <target-cli> --baseline .cliare-baseline/<target-cli>/scorecard.json --out .cliare/<target-cli> --profile deep
  6. Publish: cliare describe .cliare/<target-cli> --write && cliare report harness --out .cliare/<target-cli> --write

Measure profiles used by generated commands: `quick` is the small local smoke pass, `standard` is the normal maintainer loop, and `deep` is the broader release-quality pass for CI baselines, releases, and agent-surface publishing.

Advanced traversal knobs:
  --max-depth controls recursive command-path depth.
  --max-probes controls total runtime probes.
  --concurrency controls simultaneous probes.
  --execution-mode host measures authenticated or host-specific behavior.

Do not pass --profile to `cliare playbook`; pass it to `cliare measure` or `cliare guard`.
`.cliare/<target-cli>` is a project-scoped artifact directory, relative to the directory where you run CLIARE.
Run `cliare playbook maintainer --target <target-cli>`, `cliare playbook harness --target <target-cli>`, or `cliare playbook security --target <target-cli>` to print the full command-by-command guide."
)]
pub struct PlaybookArgs {
    /// Playbook role to print.
    #[arg(value_enum)]
    pub role: PlaybookRole,

    /// Target CLI name or path to use in generated commands.
    #[arg(long, value_name = "TARGET", value_hint = ValueHint::CommandName)]
    pub target: Option<String>,

    /// Measurement artifact directory to use in generated commands.
    #[arg(
        long,
        value_name = "DIR",
        default_value = ".cliare/<target-cli>",
        value_hint = ValueHint::DirPath
    )]
    pub out: PathBuf,

    /// Context name to use in generated report, issue, and describe commands.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = PlaybookFormat::Human)]
    pub format: PlaybookFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PlaybookRole {
    Maintainer,
    Harness,
    Security,
}

impl PlaybookRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Maintainer => "maintainer",
            Self::Harness => "harness",
            Self::Security => "security",
        }
    }
}

impl std::fmt::Display for PlaybookRole {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PlaybookFormat {
    Human,
    Markdown,
    Json,
}
