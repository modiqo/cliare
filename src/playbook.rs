use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cli::{PlaybookArgs, PlaybookFormat, PlaybookRole};
use crate::error::Result;
use crate::report_format::{escape_markdown, shell_arg};

const PLAYBOOK_SCHEMA_VERSION: &str = "cliare.playbook.v1";
const TARGET_PLACEHOLDER: &str = "<target-cli>";
const ISSUE_PLACEHOLDER: &str = "<issue-id>";

#[derive(Debug, Clone)]
pub struct PlaybookSummary {
    stdout: String,
}

impl PlaybookSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.stdout
    }
}

pub fn playbook(args: PlaybookArgs) -> Result<PlaybookSummary> {
    let packet = match args.role {
        PlaybookRole::Maintainer => MaintainerPlaybook::build(&args),
    };
    let stdout = match args.format {
        PlaybookFormat::Markdown => render_maintainer_markdown(&packet),
        PlaybookFormat::Json => format!(
            "{}\n",
            serde_json::to_string_pretty(&packet)
                .map_err(crate::error::CliareError::SerializePlaybook)?
        ),
    };
    Ok(PlaybookSummary { stdout })
}

#[derive(Debug, Serialize)]
struct MaintainerPlaybook {
    schema_version: &'static str,
    role: &'static str,
    goal: &'static str,
    target: String,
    out: PathBuf,
    context: Option<String>,
    lifecycle: Vec<PlaybookSection>,
    parameter_guide: Vec<ParameterGuide>,
    increase_budget_when: Vec<&'static str>,
    do_not_increase_budget_when: Vec<&'static str>,
    publish_artifacts: Vec<&'static str>,
    completion_criteria: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct PlaybookSection {
    order: u8,
    title: &'static str,
    purpose: &'static str,
    commands: Vec<PlaybookCommand>,
}

#[derive(Debug, Serialize)]
struct PlaybookCommand {
    title: &'static str,
    command: String,
    why: &'static str,
}

#[derive(Debug, Serialize)]
struct ParameterGuide {
    name: &'static str,
    meaning: &'static str,
    use_when: &'static str,
}

impl MaintainerPlaybook {
    fn build(args: &PlaybookArgs) -> Self {
        let target = args
            .target
            .clone()
            .unwrap_or_else(|| TARGET_PLACEHOLDER.to_owned());
        let commands = CommandBuilder::new(&target, &args.out, args.context.as_deref());
        let lifecycle = maintainer_lifecycle(&commands);

        Self {
            schema_version: PLAYBOOK_SCHEMA_VERSION,
            role: PlaybookRole::Maintainer.label(),
            goal: "Measure the CLI, inspect evidence-backed findings, fix or disposition issues, remeasure, gate in CI, and publish the agent-facing command surface.",
            target,
            out: args.out.clone(),
            context: args.context.clone(),
            lifecycle,
            parameter_guide: parameter_guide(),
            increase_budget_when: vec![
                "`budget_exhausted` is true.",
                "`frontier_remaining` is greater than zero.",
                "`observed_max_depth` equals `max_depth` and nested command families are missing.",
                "Many commands remain `candidate` instead of `runtime_confirmed`.",
                "Many machine-readable output contracts are unprobed and the missing condition is traversal budget, not a fixture.",
            ],
            do_not_increase_budget_when: vec![
                "The report says commands are blocked by auth, fixture, daemon, local repo, network, or runtime dependency preconditions.",
                "The CLI requires safe operands or sample data that CLIARE does not have.",
                "The issue is an intentional product decision that should be dispositioned.",
            ],
            publish_artifacts: vec![
                "command-index.json",
                "command-index.md",
                "issues.json",
                "issues.md",
                "issue-dispositions.json",
                "persona-harness.json",
                "persona-harness.md",
                "AGENT_SKILL.md",
                "metadata --format json command spec",
            ],
            completion_criteria: vec![
                "High severity issues are fixed, fixture-gated, dispositioned, or accepted risk.",
                "Output contracts are parse-success, documented precondition, or `needs_fixture`.",
                "Optional compatibility advisories are fixed or marked intentional/not applicable.",
                "`command-index.json` reflects the intended agent routing surface.",
                "`cliare issues list` shows reviewed decisions instead of repeated noise.",
                "CI runs `cliare measure` or `cliare guard`.",
                "Agent-facing artifacts are published or attached.",
            ],
        }
    }
}

#[derive(Debug)]
struct CommandBuilder<'a> {
    target: &'a str,
    out: &'a PathBuf,
    context: Option<&'a str>,
}

impl<'a> CommandBuilder<'a> {
    fn new(target: &'a str, out: &'a PathBuf, context: Option<&'a str>) -> Self {
        Self {
            target,
            out,
            context,
        }
    }

    fn measure(&self, profile: &str) -> String {
        format!(
            "cliare measure {} --out {} --profile {} --refresh",
            shell_token(self.target),
            shell_path(self.out),
            profile
        )
    }

    fn large_measure(&self) -> String {
        format!(
            "cliare measure {} --out {} --profile deep --max-depth 12 --max-probes 2500 --concurrency 8 --refresh",
            shell_token(self.target),
            shell_path(self.out)
        )
    }

    fn authenticated_measure(&self) -> String {
        format!(
            "cliare measure {} --out .cliare-context --context authenticated --auth-state present --execution-mode host --profile deep --refresh",
            shell_token(self.target)
        )
    }

    fn report(&self, persona: &str, extra: &[&str]) -> String {
        let mut command = format!("cliare report {} --out {}", persona, shell_path(self.out));
        self.push_context(&mut command);
        for arg in extra {
            command.push(' ');
            command.push_str(arg);
        }
        command
    }

    fn describe(&self, extra: &[&str]) -> String {
        let mut command = format!("cliare describe {}", shell_path(self.out));
        self.push_context(&mut command);
        for arg in extra {
            command.push(' ');
            command.push_str(arg);
        }
        command
    }

    fn issues_list(&self, format: &str) -> String {
        let mut command = format!("cliare issues list --out {}", shell_path(self.out));
        self.push_context(&mut command);
        command.push_str(" --format ");
        command.push_str(format);
        command
    }

    fn issues_mark(&self, status: &str, reason: &str) -> String {
        let mut command = format!(
            "cliare issues mark {} --out {}",
            ISSUE_PLACEHOLDER,
            shell_path(self.out)
        );
        self.push_context(&mut command);
        command.push_str(" --status ");
        command.push_str(status);
        command.push_str(" --reason ");
        command.push_str(&shell_arg(reason));
        command
    }

    fn guard(&self) -> String {
        format!(
            "cliare guard {} --baseline .cliare-baseline/scorecard.json --out {} --profile deep --allowed-drop 2",
            shell_token(self.target),
            shell_path(self.out)
        )
    }

    fn push_context(&self, command: &mut String) {
        if let Some(context) = self.context {
            command.push_str(" --context ");
            command.push_str(&shell_arg(context));
        }
    }
}

fn maintainer_lifecycle(commands: &CommandBuilder<'_>) -> Vec<PlaybookSection> {
    vec![
        PlaybookSection {
            order: 1,
            title: "Measure",
            purpose: "Create runtime evidence and command artifacts before reviewing issues.",
            commands: vec![
                PlaybookCommand {
                    title: "Local edit loop",
                    command: commands.measure("quick"),
                    why: "Use while changing one help path, diagnostic, or output contract.",
                },
                PlaybookCommand {
                    title: "Normal maintainer loop",
                    command: commands.measure("standard"),
                    why: "Balanced default for day-to-day maintainer work.",
                },
                PlaybookCommand {
                    title: "Release-quality pass",
                    command: commands.measure("deep"),
                    why: "Use before CI baselines, releases, and publishing agent-facing artifacts.",
                },
                PlaybookCommand {
                    title: "Very large CLI",
                    command: commands.large_measure(),
                    why: "Use only when reports show traversal pressure such as budget exhaustion or remaining frontier.",
                },
                PlaybookCommand {
                    title: "Authenticated host-context pass",
                    command: commands.authenticated_measure(),
                    why: "Use when real auth, host config, installed plugins, or local state change CLI behavior.",
                },
            ],
        },
        PlaybookSection {
            order: 2,
            title: "View",
            purpose: "Read the artifact map, maintainer report, issue ledger, and focused evidence.",
            commands: vec![
                PlaybookCommand {
                    title: "Artifact map",
                    command: commands.describe(&["--format", "markdown"]),
                    why: "Shows which artifacts exist and where to start.",
                },
                PlaybookCommand {
                    title: "Maintainer report",
                    command: commands.report("maintainer", &["--format", "markdown"]),
                    why: "Shows the maintainer action queue and reviewed decisions.",
                },
                PlaybookCommand {
                    title: "Issue ledger",
                    command: commands.issues_list("markdown"),
                    why: "Lists issues with maintainer dispositions.",
                },
                PlaybookCommand {
                    title: "Output contract drilldown",
                    command: commands.report(
                        "maintainer",
                        &["--area", "output-contracts", "--format", "markdown"],
                    ),
                    why: "Start here when machine-readable output is unprobed or failing.",
                },
                PlaybookCommand {
                    title: "Issue evidence bundle",
                    command: commands.report(
                        "maintainer",
                        &[
                            "--issue",
                            ISSUE_PLACEHOLDER,
                            "--with-evidence",
                            "--format",
                            "bundle",
                        ],
                    ),
                    why: "Use before changing or dispositioning a specific issue.",
                },
            ],
        },
        PlaybookSection {
            order: 3,
            title: "Act",
            purpose: "Fix concrete CLI contract gaps before advisory compatibility work.",
            commands: Vec::new(),
        },
        PlaybookSection {
            order: 4,
            title: "Disposition",
            purpose: "Record maintainer decisions without erasing evidence.",
            commands: vec![
                PlaybookCommand {
                    title: "Intentional behavior",
                    command: commands.issues_mark(
                        "intentional",
                        "Direct <command> --help is canonical for this CLI.",
                    ),
                    why: "Use when CLIARE observed a real behavior that is an intentional product decision.",
                },
                PlaybookCommand {
                    title: "Fixture-gated issue",
                    command: commands.issues_mark(
                        "needs-fixture",
                        "Requires safe fixture operands for <id> and <endpoint-url>.",
                    ),
                    why: "Use when the finding cannot be judged or fixed without safe operands or sample data.",
                },
                PlaybookCommand {
                    title: "Review dispositions",
                    command: commands.issues_list("markdown"),
                    why: "Confirm reviewed decisions moved out of the repeated action queue.",
                },
            ],
        },
        PlaybookSection {
            order: 5,
            title: "Remeasure",
            purpose: "Regenerate evidence after fixes or dispositions.",
            commands: vec![
                PlaybookCommand {
                    title: "Deep rerun",
                    command: commands.measure("deep"),
                    why: "Use after implementation fixes or fixture additions.",
                },
                PlaybookCommand {
                    title: "Persist reports",
                    command: commands.report("maintainer", &["--write"]),
                    why: "Writes persona reports and issue ledger artifacts.",
                },
                PlaybookCommand {
                    title: "Verify remaining issues",
                    command: commands.issues_list("markdown"),
                    why: "Confirm action-required and reviewed-decision counts.",
                },
            ],
        },
        PlaybookSection {
            order: 6,
            title: "Gate in CI",
            purpose: "Prevent readiness regressions after a baseline exists.",
            commands: vec![PlaybookCommand {
                title: "Score guard",
                command: commands.guard(),
                why: "Fails when score drops beyond the allowed threshold or policy checks fail.",
            }],
        },
        PlaybookSection {
            order: 7,
            title: "Publish Agent Surface",
            purpose: "Publish the command index, harness report, dispositions, and skills that agents should read before invoking the target CLI.",
            commands: vec![
                PlaybookCommand {
                    title: "Artifact navigation",
                    command: commands.describe(&["--write"]),
                    why: "Writes artifact map files for humans and agents.",
                },
                PlaybookCommand {
                    title: "Harness packet",
                    command: commands.report("harness", &["--write"]),
                    why: "Writes the agent harness view over the measured command surface.",
                },
                PlaybookCommand {
                    title: "Install review skills",
                    command: "cliare skills install --agent all --scope project".to_owned(),
                    why: "Installs local CLIARE artifact-review skills for supported agents.",
                },
                PlaybookCommand {
                    title: "CLIARE command spec",
                    command: "cliare metadata --format json".to_owned(),
                    why: "Publishes CLIARE's own command contract for agents.",
                },
            ],
        },
    ]
}

fn parameter_guide() -> Vec<ParameterGuide> {
    vec![
        ParameterGuide {
            name: "--profile quick",
            meaning: "Small local smoke pass.",
            use_when: "Editing help, diagnostics, or one output contract.",
        },
        ParameterGuide {
            name: "--profile standard",
            meaning: "Balanced default pass.",
            use_when: "Normal maintainer loop.",
        },
        ParameterGuide {
            name: "--profile deep",
            meaning: "Broader release-quality pass.",
            use_when: "CI baseline, release, or publishing agent surface.",
        },
        ParameterGuide {
            name: "--max-depth",
            meaning: "Recursive command-path depth.",
            use_when: "Nested command families are missing or observed_max_depth equals max_depth.",
        },
        ParameterGuide {
            name: "--max-probes",
            meaning: "Maximum runtime probes.",
            use_when: "budget_exhausted is true, frontier_remaining is greater than zero, or too many candidate commands remain.",
        },
        ParameterGuide {
            name: "--concurrency",
            meaning: "Probes run at the same time.",
            use_when: "Lower for rate limits, shared state, daemons, or flaky CLIs; raise only for stable local CLIs.",
        },
        ParameterGuide {
            name: "--timeout-ms",
            meaning: "Per-probe timeout.",
            use_when: "The CLI is slow, network-backed, daemon-backed, or package-manager-like.",
        },
        ParameterGuide {
            name: "--output-limit-bytes",
            meaning: "Retained stdout/stderr bytes per probe.",
            use_when: "Help or machine output is legitimately large.",
        },
        ParameterGuide {
            name: "--execution-mode isolated",
            meaning: "Default sandboxed profile.",
            use_when: "Safe local probing.",
        },
        ParameterGuide {
            name: "--execution-mode host",
            meaning: "Host config, auth, plugins, and local state are visible.",
            use_when: "Measuring authenticated or host-specific behavior.",
        },
    ]
}

fn render_maintainer_markdown(playbook: &MaintainerPlaybook) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# CLIARE Maintainer Playbook").expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(&mut text, "{}", escape_markdown(playbook.goal))
        .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "| Field | Value |\n|---|---|\n| Target | `{}` |\n| Artifact dir | `{}` |\n| Context | `{}` |",
        escape_markdown(&playbook.target),
        playbook.out.display(),
        playbook.context.as_deref().unwrap_or("none")
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");

    for section in &playbook.lifecycle {
        writeln!(
            &mut text,
            "## {}. {}",
            section.order,
            escape_markdown(section.title)
        )
        .expect("writing to string cannot fail");
        writeln!(&mut text).expect("writing to string cannot fail");
        writeln!(&mut text, "{}", escape_markdown(section.purpose))
            .expect("writing to string cannot fail");
        writeln!(&mut text).expect("writing to string cannot fail");
        if section.title == "Act" {
            render_triage_order(&mut text);
        }
        for command in &section.commands {
            writeln!(&mut text, "### {}", escape_markdown(command.title))
                .expect("writing to string cannot fail");
            writeln!(&mut text).expect("writing to string cannot fail");
            writeln!(&mut text, "{}", escape_markdown(command.why))
                .expect("writing to string cannot fail");
            writeln!(&mut text).expect("writing to string cannot fail");
            writeln!(&mut text, "```sh").expect("writing to string cannot fail");
            writeln!(&mut text, "{}", command.command).expect("writing to string cannot fail");
            writeln!(&mut text, "```").expect("writing to string cannot fail");
            writeln!(&mut text).expect("writing to string cannot fail");
        }
    }

    render_parameter_guide(&mut text, playbook);
    render_publish_artifacts(&mut text, playbook);
    render_completion_criteria(&mut text, playbook);

    text
}

fn render_triage_order(text: &mut String) {
    writeln!(text, "Triage in this order:").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    let rows = [
        (
            "Output contracts",
            "parseable JSON/YAML, safe dry-run behavior, fixture paths",
        ),
        (
            "Preconditions",
            "auth, local context, daemon, network, runtime dependency, fixture requirements",
        ),
        (
            "Command discovery",
            "command-specific --help and stable usage syntax",
        ),
        ("Diagnostics", "invalid command and invalid flag recovery"),
        (
            "Safety",
            "discovery-time side effects and credential-like paths",
        ),
        (
            "Compatibility advisories",
            "optional conventions such as help <path>",
        ),
    ];
    for (index, (title, detail)) in rows.iter().enumerate() {
        writeln!(text, "{}. {}: {}.", index + 1, title, detail)
            .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_parameter_guide(text: &mut String, playbook: &MaintainerPlaybook) {
    writeln!(text, "## Parameter Guide").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Most maintainers should choose only `quick`, `standard`, or `deep`. Change advanced parameters only when the report points to traversal pressure."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Parameter | Meaning | Use When |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|---|").expect("writing to string cannot fail");
    for parameter in &playbook.parameter_guide {
        writeln!(
            text,
            "| `{}` | {} | {} |",
            parameter.name,
            escape_markdown(parameter.meaning),
            escape_markdown(parameter.use_when)
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");

    writeln!(text, "Increase depth or probes when:").expect("writing to string cannot fail");
    for item in &playbook.increase_budget_when {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Do not increase budget when:").expect("writing to string cannot fail");
    for item in &playbook.do_not_increase_budget_when {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_publish_artifacts(text: &mut String, playbook: &MaintainerPlaybook) {
    writeln!(text, "## Agent-Facing Artifacts").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Publish or attach these so agent harnesses can route deliberately instead of rediscovering syntax by trial and error:"
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for artifact in &playbook.publish_artifacts {
        writeln!(text, "- `{}`", artifact).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_completion_criteria(text: &mut String, playbook: &MaintainerPlaybook) {
    writeln!(text, "## Completion Criteria").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for item in &playbook.completion_criteria {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
}

fn shell_path(path: &Path) -> String {
    shell_arg(&path.display().to_string())
}

fn shell_token(value: &str) -> String {
    if value == TARGET_PLACEHOLDER {
        value.to_owned()
    } else {
        shell_arg(value)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::{PlaybookArgs, PlaybookFormat, PlaybookRole};

    use super::{MaintainerPlaybook, playbook};

    #[test]
    fn maintainer_playbook_contains_full_lifecycle() {
        let args = PlaybookArgs {
            role: PlaybookRole::Maintainer,
            target: Some("rote".to_owned()),
            out: PathBuf::from(".cliare"),
            context: None,
            format: PlaybookFormat::Markdown,
        };

        let summary = playbook(args).expect("playbook renders");
        let text = summary.terminal_summary();

        assert!(text.contains("## 1. Measure"));
        assert!(text.contains("## 2. View"));
        assert!(text.contains("## 4. Disposition"));
        assert!(text.contains("## 6. Gate in CI"));
        assert!(text.contains("## 7. Publish Agent Surface"));
        assert!(text.contains("cliare measure rote --out .cliare --profile deep --refresh"));
        assert!(text.contains("cliare issues list --out .cliare --format markdown"));
    }

    #[test]
    fn maintainer_playbook_json_is_structured() {
        let args = PlaybookArgs {
            role: PlaybookRole::Maintainer,
            target: None,
            out: PathBuf::from(".cliare"),
            context: Some("authenticated".to_owned()),
            format: PlaybookFormat::Json,
        };

        let summary = playbook(args).expect("playbook renders");
        let value: serde_json::Value =
            serde_json::from_str(summary.terminal_summary()).expect("json parses");

        assert_eq!(value["schema_version"], "cliare.playbook.v1");
        assert_eq!(value["role"], "maintainer");
        assert_eq!(value["target"], "<target-cli>");
        assert_eq!(value["context"], "authenticated");
    }

    #[test]
    fn maintainer_playbook_uses_context_for_view_commands() {
        let args = PlaybookArgs {
            role: PlaybookRole::Maintainer,
            target: Some("rote".to_owned()),
            out: PathBuf::from(".cliare-context"),
            context: Some("authenticated".to_owned()),
            format: PlaybookFormat::Markdown,
        };

        let packet = MaintainerPlaybook::build(&args);
        let report_command = packet.lifecycle[1].commands[1].command.as_str();

        assert!(report_command.contains("--context authenticated"));
    }
}
