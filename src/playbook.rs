use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cli::{PlaybookArgs, PlaybookFormat, PlaybookRole};
use crate::error::Result;
use crate::report_format::{escape_markdown, shell_arg};

const PLAYBOOK_SCHEMA_VERSION: &str = "cliare.playbook.v1";
const TARGET_PLACEHOLDER: &str = "<target-cli>";
const ISSUE_PLACEHOLDER: &str = "<issue-id>";
const DEFAULT_OUT_PLACEHOLDER: &str = ".cliare/<target-cli>";

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
        PlaybookRole::Maintainer => RolePlaybook::build_maintainer(&args),
        PlaybookRole::Harness => RolePlaybook::build_harness(&args),
        PlaybookRole::Security => RolePlaybook::build_security(&args),
    };
    let stdout = match args.format {
        PlaybookFormat::Human => render_human(&packet),
        PlaybookFormat::Markdown => render_markdown(&packet),
        PlaybookFormat::Json => format!(
            "{}\n",
            serde_json::to_string_pretty(&packet)
                .map_err(crate::error::CliareError::SerializePlaybook)?
        ),
    };
    Ok(PlaybookSummary { stdout })
}

#[derive(Debug, Serialize)]
struct RolePlaybook {
    schema_version: &'static str,
    role: &'static str,
    title: &'static str,
    goal: &'static str,
    target: String,
    out: PathBuf,
    context: Option<String>,
    artifact_layout: Vec<&'static str>,
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

impl RolePlaybook {
    fn build_maintainer(args: &PlaybookArgs) -> Self {
        let target = args
            .target
            .clone()
            .unwrap_or_else(|| TARGET_PLACEHOLDER.to_owned());
        let out = effective_artifact_dir(args, &target);
        let commands = CommandBuilder::new(&target, &out, args.context.as_deref());
        let lifecycle = maintainer_lifecycle(&commands);

        Self {
            schema_version: PLAYBOOK_SCHEMA_VERSION,
            role: PlaybookRole::Maintainer.label(),
            title: "CLIARE Maintainer Playbook",
            goal: "Measure the CLI, inspect evidence-backed findings, fix or disposition issues, remeasure, gate in CI, and publish the agent-facing command surface.",
            target,
            out,
            context: args.context.clone(),
            artifact_layout: artifact_layout(),
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

    fn build_harness(args: &PlaybookArgs) -> Self {
        let target = args
            .target
            .clone()
            .unwrap_or_else(|| TARGET_PLACEHOLDER.to_owned());
        let out = effective_artifact_dir(args, &target);
        let commands = CommandBuilder::new(&target, &out, args.context.as_deref());
        let lifecycle = harness_lifecycle(&commands);

        Self {
            schema_version: PLAYBOOK_SCHEMA_VERSION,
            role: PlaybookRole::Harness.label(),
            title: "CLIARE Harness Playbook",
            goal: "Consume CLIARE artifacts as an agent-routing contract: command index first, harness packet second, generated skill third, then validate agent behavior against evidence.",
            target,
            out,
            context: args.context.clone(),
            artifact_layout: artifact_layout(),
            lifecycle,
            parameter_guide: parameter_guide(),
            increase_budget_when: vec![
                "The command index still has many candidate commands that agents need.",
                "The harness packet reports traversal pressure or missing nested command families.",
                "The generated skill lacks enough runtime-confirmed commands for the intended workflows.",
            ],
            do_not_increase_budget_when: vec![
                "A command is blocked by auth, fixture data, local repository context, daemon state, network, or runtime dependencies.",
                "The harness needs a curated routing policy rather than more command discovery.",
                "The command is intentionally not part of the agent-facing surface.",
            ],
            publish_artifacts: vec![
                "command-index.json",
                "command-index.md",
                "persona-harness.json",
                "persona-harness.md",
                "AGENT_SKILL.md",
                "artifact-map.json",
                "artifact-map.md",
                "issue-dispositions.json",
            ],
            completion_criteria: vec![
                "Agent routing starts from `command-index.json`, not from ad hoc help probing.",
                "The harness packet identifies safe commands, preconditions, output contracts, and known gaps.",
                "The generated skill explains how to select commands, handle fixtures, and avoid unsafe probes.",
                "Critical harness issues are fixed, dispositioned, or excluded from the agent-facing surface.",
                "CI or release automation regenerates artifacts when the CLI surface changes.",
            ],
        }
    }

    fn build_security(args: &PlaybookArgs) -> Self {
        let target = args
            .target
            .clone()
            .unwrap_or_else(|| TARGET_PLACEHOLDER.to_owned());
        let out = effective_artifact_dir(args, &target);
        let commands = CommandBuilder::new(&target, &out, args.context.as_deref());
        let lifecycle = security_lifecycle(&commands);

        Self {
            schema_version: PLAYBOOK_SCHEMA_VERSION,
            role: PlaybookRole::Security.label(),
            title: "CLIARE Security Playbook",
            goal: "Review CLIARE evidence for safe agent use: side effects, credential-like paths, host/auth exposure, preconditions, policy gates, and publishable caveats.",
            target,
            out,
            context: args.context.clone(),
            artifact_layout: artifact_layout(),
            lifecycle,
            parameter_guide: parameter_guide(),
            increase_budget_when: vec![
                "Safety or execution findings are inconclusive because traversal stopped early.",
                "Security-relevant command families are still candidates instead of runtime-confirmed.",
                "A controlled host-context pass is required to verify authenticated behavior.",
            ],
            do_not_increase_budget_when: vec![
                "The issue requires credentials, fixtures, or production-like data that should not be probed automatically.",
                "The finding is a policy decision that needs an explicit disposition.",
                "The next step is manual review of evidence paths or side-effect traces.",
            ],
            publish_artifacts: vec![
                "persona-security.json",
                "persona-security.md",
                "issues.json",
                "issues.md",
                "issue-dispositions.json",
                "scorecard.json",
                "evidence.jsonl",
                "command-index.json",
            ],
            completion_criteria: vec![
                "Credential-like and persistent side effects are fixed, allowlisted by policy, or accepted with a written disposition.",
                "Host/authenticated measurements are clearly separated from isolated measurements.",
                "Preconditions are explicit enough that agent harnesses can avoid unsafe retries.",
                "Security-relevant issues have evidence bundles or reviewed dispositions.",
                "A guard or policy check exists before publishing agent-facing artifacts.",
            ],
        }
    }
}

#[derive(Debug)]
struct CommandBuilder<'a> {
    target: &'a str,
    out: &'a Path,
    context: Option<&'a str>,
}

impl<'a> CommandBuilder<'a> {
    fn new(target: &'a str, out: &'a Path, context: Option<&'a str>) -> Self {
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
            "cliare measure {} --out {} --profile deep --max-depth 12 --max-probes 5000 --concurrency 8 --refresh",
            shell_token(self.target),
            shell_path(self.out)
        )
    }

    fn detached_measure(&self) -> String {
        format!("{} --detach", self.large_measure())
    }

    fn authenticated_measure(&self) -> String {
        format!(
            "cliare measure {} --out {} --context authenticated --auth-state present --execution-mode host --profile deep --refresh",
            shell_token(self.target),
            shell_path(self.out)
        )
    }

    fn job_status(&self) -> String {
        let mut command = format!("cliare jobs status --out {}", shell_path(self.out));
        self.push_context(&mut command);
        command
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

    fn skills_install(&self) -> String {
        "cliare skills install --agent all --scope project".to_owned()
    }

    fn metadata_json(&self) -> String {
        "cliare metadata --format json".to_owned()
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
            "cliare guard {} --baseline {} --out {} --profile deep --allowed-drop 2",
            shell_token(self.target),
            shell_path(&baseline_scorecard_path(self.target)),
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
                    title: "Detached long run",
                    command: commands.detached_measure(),
                    why: "Use when a deep run may take long enough that you want to continue in the background.",
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
                    title: "Detached job status",
                    command: commands.job_status(),
                    why: "Use after `measure --detach`; wait for `complete` before reading reports or issues.",
                },
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
                    command: commands.skills_install(),
                    why: "Installs local CLIARE artifact-review skills for supported agents.",
                },
                PlaybookCommand {
                    title: "CLIARE command spec",
                    command: commands.metadata_json(),
                    why: "Publishes CLIARE's own command contract for agents.",
                },
            ],
        },
    ]
}

fn harness_lifecycle(commands: &CommandBuilder<'_>) -> Vec<PlaybookSection> {
    vec![
        PlaybookSection {
            order: 1,
            title: "Acquire Artifacts",
            purpose: "Start from a fresh CLIARE run or a maintainer-published artifact directory.",
            commands: vec![
                PlaybookCommand {
                    title: "Release-quality measurement",
                    command: commands.measure("deep"),
                    why: "Use when the harness team owns measurement for the target CLI.",
                },
                PlaybookCommand {
                    title: "Detached long run",
                    command: commands.detached_measure(),
                    why: "Use for large command surfaces, then wait for completion before consuming artifacts.",
                },
                PlaybookCommand {
                    title: "Detached job status",
                    command: commands.job_status(),
                    why: "Confirms whether artifacts are ready for agent consumption.",
                },
                PlaybookCommand {
                    title: "Artifact map",
                    command: commands.describe(&["--format", "markdown"]),
                    why: "Shows exactly where the command index, harness packet, and skill live.",
                },
            ],
        },
        PlaybookSection {
            order: 2,
            title: "Read Agent Surface",
            purpose: "Use the command index and harness persona packet as the routing contract.",
            commands: vec![
                PlaybookCommand {
                    title: "Harness packet",
                    command: commands.report("harness", &["--format", "markdown"]),
                    why: "Summarizes what agents can safely route through and what requires preconditions.",
                },
                PlaybookCommand {
                    title: "Command index map",
                    command: commands.describe(&["--format", "json"]),
                    why: "Lets automation locate `command-index.json`, `AGENT_SKILL.md`, and reports.",
                },
                PlaybookCommand {
                    title: "Harness output contracts",
                    command: commands.report(
                        "harness",
                        &["--area", "output-contracts", "--format", "markdown"],
                    ),
                    why: "Use this before wiring structured output parsing in an agent harness.",
                },
                PlaybookCommand {
                    title: "Harness safety view",
                    command: commands
                        .report("harness", &["--area", "safety", "--format", "markdown"]),
                    why: "Use this before allowing agents to run probes or operational commands.",
                },
            ],
        },
        PlaybookSection {
            order: 3,
            title: "Install Skill",
            purpose: "Make the measured CLI behavior available where agents look for operational instructions.",
            commands: vec![
                PlaybookCommand {
                    title: "Install local review skills",
                    command: commands.skills_install(),
                    why: "Installs CLIARE artifact-review skills for supported local agent harnesses.",
                },
                PlaybookCommand {
                    title: "Write harness artifacts",
                    command: commands.report("harness", &["--write"]),
                    why: "Persists the harness packet and generated agent-facing artifacts.",
                },
                PlaybookCommand {
                    title: "Write artifact map",
                    command: commands.describe(&["--write"]),
                    why: "Persists navigation files that agents can consume without guessing paths.",
                },
            ],
        },
        PlaybookSection {
            order: 4,
            title: "Configure Routing",
            purpose: "Teach the harness to prefer runtime-confirmed commands and to respect documented preconditions.",
            commands: Vec::new(),
        },
        PlaybookSection {
            order: 5,
            title: "Review Gaps",
            purpose: "Escalate gaps back to maintainers instead of letting agents rediscover syntax by trial and error.",
            commands: vec![
                PlaybookCommand {
                    title: "Issue ledger",
                    command: commands.issues_list("human"),
                    why: "Shows the action queue in a concise review format.",
                },
                PlaybookCommand {
                    title: "Issue evidence bundle",
                    command: commands.report(
                        "harness",
                        &[
                            "--issue",
                            ISSUE_PLACEHOLDER,
                            "--with-evidence",
                            "--format",
                            "bundle",
                        ],
                    ),
                    why: "Attach this when filing harness-blocking CLI issues.",
                },
            ],
        },
        PlaybookSection {
            order: 6,
            title: "Publish for Agents",
            purpose: "Publish the command index, harness packet, and skill together so agents have one evidence-backed source of truth.",
            commands: vec![
                PlaybookCommand {
                    title: "Publish harness packet",
                    command: commands.report("harness", &["--write"]),
                    why: "Writes files that agent harnesses should read before invoking the target CLI.",
                },
                PlaybookCommand {
                    title: "CLIARE command spec",
                    command: commands.metadata_json(),
                    why: "Exposes CLIARE's own command surface to agent harnesses.",
                },
            ],
        },
    ]
}

fn security_lifecycle(commands: &CommandBuilder<'_>) -> Vec<PlaybookSection> {
    vec![
        PlaybookSection {
            order: 1,
            title: "Measure Safely",
            purpose: "Start with isolated measurement; add host or authenticated context only when reviewing a controlled environment.",
            commands: vec![
                PlaybookCommand {
                    title: "Isolated safety pass",
                    command: commands.measure("standard"),
                    why: "Default security review pass for discovery-time side effects and CLI shape.",
                },
                PlaybookCommand {
                    title: "Deep isolated pass",
                    command: commands.measure("deep"),
                    why: "Use before approval when safety-relevant command families are still shallow or candidate-only.",
                },
                PlaybookCommand {
                    title: "Authenticated host-context pass",
                    command: commands.authenticated_measure(),
                    why: "Use only in a controlled environment when auth, plugins, or host config change behavior.",
                },
            ],
        },
        PlaybookSection {
            order: 2,
            title: "Review Security Evidence",
            purpose: "Inspect the security packet, safety area, preconditions, and policy impact before approving agent use.",
            commands: vec![
                PlaybookCommand {
                    title: "Security packet",
                    command: commands.report("security", &["--format", "markdown"]),
                    why: "Shows security-oriented findings and recommendations.",
                },
                PlaybookCommand {
                    title: "Safety drilldown",
                    command: commands
                        .report("security", &["--area", "safety", "--format", "markdown"]),
                    why: "Focuses on side effects and credential-like filesystem evidence.",
                },
                PlaybookCommand {
                    title: "Precondition drilldown",
                    command: commands.report(
                        "security",
                        &["--area", "preconditions", "--format", "markdown"],
                    ),
                    why: "Separates auth, fixture, daemon, local repo, network, and runtime dependency blockers.",
                },
                PlaybookCommand {
                    title: "Policy drilldown",
                    command: commands
                        .report("security", &["--area", "policy", "--format", "markdown"]),
                    why: "Use when CI gates or security policy thresholds are involved.",
                },
            ],
        },
        PlaybookSection {
            order: 3,
            title: "Inspect Evidence",
            purpose: "Attach evidence before accepting risk, allowlisting side effects, or escalating to maintainers.",
            commands: vec![
                PlaybookCommand {
                    title: "Issue ledger",
                    command: commands.issues_list("human"),
                    why: "Shows open and reviewed security-relevant decisions.",
                },
                PlaybookCommand {
                    title: "Issue evidence bundle",
                    command: commands.report(
                        "security",
                        &[
                            "--issue",
                            ISSUE_PLACEHOLDER,
                            "--with-evidence",
                            "--format",
                            "bundle",
                        ],
                    ),
                    why: "Use before writing an exception, policy allowlist, or bug report.",
                },
            ],
        },
        PlaybookSection {
            order: 4,
            title: "Decide",
            purpose: "Record whether the behavior is fixed, intentionally allowed, fixture-gated, not applicable, deferred, or accepted risk.",
            commands: vec![
                PlaybookCommand {
                    title: "Accepted risk",
                    command: commands.issues_mark(
                        "accepted-risk",
                        "Security reviewed evidence and accepted the documented residual risk.",
                    ),
                    why: "Use when the finding is real but approved for agent use under constraints.",
                },
                PlaybookCommand {
                    title: "Not applicable",
                    command: commands.issues_mark(
                        "not-applicable",
                        "Finding does not apply to the approved deployment profile.",
                    ),
                    why: "Use when the evidence is valid but out of scope for this environment.",
                },
                PlaybookCommand {
                    title: "Review decisions",
                    command: commands.issues_list("human"),
                    why: "Confirm decisions are visible and no longer repeat as unreviewed action items.",
                },
            ],
        },
        PlaybookSection {
            order: 5,
            title: "Gate and Publish",
            purpose: "Keep security decisions attached to the same artifact set agents and maintainers consume.",
            commands: vec![
                PlaybookCommand {
                    title: "Score guard",
                    command: commands.guard(),
                    why: "Use after a baseline exists to catch score and policy regressions.",
                },
                PlaybookCommand {
                    title: "Write security packet",
                    command: commands.report("security", &["--write"]),
                    why: "Persists security review output beside the measured artifacts.",
                },
                PlaybookCommand {
                    title: "Write artifact map",
                    command: commands.describe(&["--write"]),
                    why: "Publishes navigation files with security and agent-facing artifacts.",
                },
            ],
        },
    ]
}

fn artifact_layout() -> Vec<&'static str> {
    vec![
        "`--out` names one target's artifact root, not a global CLIARE database.",
        "Use `.cliare/<target-cli>` for normal project-local runs.",
        "Context runs write under `.cliare/<target-cli>/contexts/<context>`.",
        "If you use `--detach`, wait for `cliare jobs status --out <artifact-dir>` to report `complete` before reading reports or issues.",
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

fn render_markdown(playbook: &RolePlaybook) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# {}", playbook.title).expect("writing to string cannot fail");
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
    render_artifact_layout(&mut text, playbook);

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

fn render_human(playbook: &RolePlaybook) -> String {
    if playbook.role != PlaybookRole::Maintainer.label() {
        return render_role_human(playbook);
    }

    let mut text = String::new();
    writeln!(text, "CLIARE maintainer walkthrough").expect("writing to string cannot fail");
    writeln!(text, "target: {}", playbook.target).expect("writing to string cannot fail");
    writeln!(text, "artifacts: {}", playbook.out.display()).expect("writing to string cannot fail");
    if let Some(context) = &playbook.context {
        writeln!(text, "context: {context}").expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Read this as a checklist. Run one measure command, wait for it to finish, inspect issues, then fix or disposition before rerunning."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Artifact rule").expect("writing to string cannot fail");
    writeln!(
        text,
        "  {} is this target's artifact root. It is relative to your current directory.",
        playbook.out.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "  Do not use bare .cliare when .cliare contains multiple target folders."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    render_human_step(
        &mut text,
        1,
        "Measure",
        "Use standard for normal work. Use deep for release or launch-quality review.",
        &[
            (
                "normal",
                required_command(playbook, "Measure", "Normal maintainer loop"),
            ),
            (
                "quick edit loop",
                required_command(playbook, "Measure", "Local edit loop"),
            ),
            (
                "deep release pass",
                required_command(playbook, "Measure", "Release-quality pass"),
            ),
            (
                "large CLI",
                required_command(playbook, "Measure", "Very large CLI"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        2,
        "For long runs",
        "Detach only when you do not want to block the terminal. Do not read reports until the job is complete.",
        &[
            (
                "start detached",
                required_command(playbook, "Measure", "Detached long run"),
            ),
            (
                "check status",
                required_command(playbook, "View", "Detached job status"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        3,
        "Inspect",
        "Start with the issue list, then open the maintainer report or focused evidence when a row needs explanation.",
        &[
            ("issues", required_command(playbook, "View", "Issue ledger")),
            (
                "maintainer report",
                required_command(playbook, "View", "Maintainer report"),
            ),
            (
                "output contracts",
                required_command(playbook, "View", "Output contract drilldown"),
            ),
            (
                "one issue with evidence",
                required_command(playbook, "View", "Issue evidence bundle"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        4,
        "Act",
        "Fix real CLI contract gaps first: output contracts, preconditions, command-specific help, diagnostics, and safety.",
        &[],
    );
    render_human_step(
        &mut text,
        5,
        "Disposition what is not a bug",
        "Use a disposition when the finding is intentional, fixture-gated, not applicable, false positive, deferred, or accepted risk.",
        &[
            (
                "intentional behavior",
                required_command(playbook, "Disposition", "Intentional behavior"),
            ),
            (
                "needs fixture",
                required_command(playbook, "Disposition", "Fixture-gated issue"),
            ),
            (
                "review queue",
                required_command(playbook, "Disposition", "Review dispositions"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        6,
        "Remeasure",
        "After fixes or dispositions, regenerate evidence and verify that repeated noise dropped.",
        &[
            (
                "rerun",
                required_command(playbook, "Remeasure", "Deep rerun"),
            ),
            (
                "write reports",
                required_command(playbook, "Remeasure", "Persist reports"),
            ),
            (
                "verify",
                required_command(playbook, "Remeasure", "Verify remaining issues"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        7,
        "Gate and publish",
        "Use a guard once a baseline exists, then publish the command index and harness packet for agents.",
        &[
            (
                "CI guard",
                required_command(playbook, "Gate in CI", "Score guard"),
            ),
            (
                "artifact map",
                required_command(playbook, "Publish Agent Surface", "Artifact navigation"),
            ),
            (
                "agent harness packet",
                required_command(playbook, "Publish Agent Surface", "Harness packet"),
            ),
        ],
    );

    writeln!(text, "Rules of thumb").expect("writing to string cannot fail");
    writeln!(
        text,
        "  Increase --max-depth or --max-probes only when the report shows traversal pressure."
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "  Do not increase probe budget for auth, fixture, daemon, repo, network, or runtime-dependency preconditions."
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "  For authenticated behavior, measure the same artifact root with --context authenticated."
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "  Use --format markdown for a full document or --format json for automation."
    )
    .expect("writing to string cannot fail");

    text
}

fn render_role_human(playbook: &RolePlaybook) -> String {
    let mut text = String::new();
    writeln!(text, "CLIARE {} walkthrough", playbook.role).expect("writing to string cannot fail");
    writeln!(text, "target: {}", playbook.target).expect("writing to string cannot fail");
    writeln!(text, "artifacts: {}", playbook.out.display()).expect("writing to string cannot fail");
    if let Some(context) = &playbook.context {
        writeln!(text, "context: {context}").expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Goal").expect("writing to string cannot fail");
    writeln!(text, "  {}", playbook.goal).expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Artifact rule").expect("writing to string cannot fail");
    writeln!(
        text,
        "  {} is this target's artifact root, relative to your current directory.",
        playbook.out.display()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    for section in &playbook.lifecycle {
        render_human_step_from_section(&mut text, section);
    }

    writeln!(text, "Completion criteria").expect("writing to string cannot fail");
    for item in &playbook.completion_criteria {
        writeln!(text, "  - {item}").expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Use --format markdown for the full document or --format json for automation."
    )
    .expect("writing to string cannot fail");

    text
}

fn render_human_step_from_section(text: &mut String, section: &PlaybookSection) {
    writeln!(text, "{}. {}", section.order, section.title).expect("writing to string cannot fail");
    writeln!(text, "   {}", section.purpose).expect("writing to string cannot fail");
    if section.commands.is_empty() {
        writeln!(
            text,
            "   Review the generated artifacts and apply the guidance manually."
        )
        .expect("writing to string cannot fail");
    }
    for command in &section.commands {
        writeln!(text, "   {}:", command.title).expect("writing to string cannot fail");
        writeln!(text, "     {}", command.command).expect("writing to string cannot fail");
        writeln!(text, "     {}", command.why).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_human_step(
    text: &mut String,
    number: u8,
    title: &str,
    guidance: &str,
    commands: &[(&str, &str)],
) {
    writeln!(text, "{number}. {title}").expect("writing to string cannot fail");
    writeln!(text, "   {guidance}").expect("writing to string cannot fail");
    for (label, command) in commands {
        writeln!(text, "   {label}:").expect("writing to string cannot fail");
        writeln!(text, "     {command}").expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn required_command<'a>(
    playbook: &'a RolePlaybook,
    section_title: &str,
    command_title: &str,
) -> &'a str {
    playbook
        .lifecycle
        .iter()
        .find(|section| section.title == section_title)
        .and_then(|section| {
            section
                .commands
                .iter()
                .find(|command| command.title == command_title)
        })
        .map(|command| command.command.as_str())
        .unwrap_or("missing playbook command")
}

fn render_artifact_layout(text: &mut String, playbook: &RolePlaybook) {
    writeln!(text, "## Artifact Directory").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for item in &playbook.artifact_layout {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
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

fn render_parameter_guide(text: &mut String, playbook: &RolePlaybook) {
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

fn render_publish_artifacts(text: &mut String, playbook: &RolePlaybook) {
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

fn render_completion_criteria(text: &mut String, playbook: &RolePlaybook) {
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

fn effective_artifact_dir(args: &PlaybookArgs, target: &str) -> PathBuf {
    if args.out == Path::new(DEFAULT_OUT_PLACEHOLDER) {
        if target == TARGET_PLACEHOLDER {
            PathBuf::from(DEFAULT_OUT_PLACEHOLDER)
        } else {
            PathBuf::from(".cliare").join(artifact_dir_segment(target))
        }
    } else {
        args.out.clone()
    }
}

fn artifact_dir_segment(target: &str) -> String {
    let raw = Path::new(target)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(target);
    let mut segment = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            segment.push(ch);
        } else if !segment.ends_with('-') {
            segment.push('-');
        }
    }
    let segment = segment.trim_matches('-');
    if segment.is_empty() {
        "target-cli".to_owned()
    } else {
        segment.to_owned()
    }
}

fn baseline_scorecard_path(target: &str) -> PathBuf {
    if target == TARGET_PLACEHOLDER {
        PathBuf::from(".cliare-baseline")
            .join(TARGET_PLACEHOLDER)
            .join("scorecard.json")
    } else {
        PathBuf::from(".cliare-baseline")
            .join(artifact_dir_segment(target))
            .join("scorecard.json")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::{PlaybookArgs, PlaybookFormat, PlaybookRole};

    use super::{DEFAULT_OUT_PLACEHOLDER, RolePlaybook, playbook};

    #[test]
    fn maintainer_playbook_contains_full_lifecycle() {
        let args = PlaybookArgs {
            role: PlaybookRole::Maintainer,
            target: Some("rote".to_owned()),
            out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
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
        assert!(text.contains("## Artifact Directory"));
        assert!(text.contains("cliare measure rote --out .cliare/rote --profile deep --refresh"));
        assert!(text.contains("cliare jobs status --out .cliare/rote"));
        assert!(text.contains("cliare issues list --out .cliare/rote --format markdown"));
        assert!(text.contains("cliare measure rote --out .cliare/rote --context authenticated"));
    }

    #[test]
    fn maintainer_playbook_json_is_structured() {
        let args = PlaybookArgs {
            role: PlaybookRole::Maintainer,
            target: None,
            out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
            context: Some("authenticated".to_owned()),
            format: PlaybookFormat::Json,
        };

        let summary = playbook(args).expect("playbook renders");
        let value: serde_json::Value =
            serde_json::from_str(summary.terminal_summary()).expect("json parses");

        assert_eq!(value["schema_version"], "cliare.playbook.v1");
        assert_eq!(value["role"], "maintainer");
        assert_eq!(value["title"], "CLIARE Maintainer Playbook");
        assert_eq!(value["target"], "<target-cli>");
        assert_eq!(value["out"], ".cliare/<target-cli>");
        assert_eq!(value["context"], "authenticated");
        assert!(value["artifact_layout"].is_array());
    }

    #[test]
    fn maintainer_playbook_human_is_step_by_step() {
        let args = PlaybookArgs {
            role: PlaybookRole::Maintainer,
            target: Some("mise".to_owned()),
            out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
            context: None,
            format: PlaybookFormat::Human,
        };

        let summary = playbook(args).expect("playbook renders");
        let text = summary.terminal_summary();

        assert!(text.contains("CLIARE maintainer walkthrough"));
        assert!(text.contains("artifacts: .cliare/mise"));
        assert!(text.contains("1. Measure"));
        assert!(text.contains("2. For long runs"));
        assert!(text.contains("cliare jobs status --out .cliare/mise"));
        assert!(text.contains("cliare issues list --out .cliare/mise --format markdown"));
        assert!(text.contains("Rules of thumb"));
        assert!(!text.contains("| Field | Value |"));
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

        let packet = RolePlaybook::build_maintainer(&args);
        let report_command = packet.lifecycle[1].commands[1].command.as_str();

        assert!(report_command.contains("--context authenticated"));
    }

    #[test]
    fn harness_playbook_contains_agent_execution_loop() {
        let args = PlaybookArgs {
            role: PlaybookRole::Harness,
            target: Some("rote".to_owned()),
            out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
            context: None,
            format: PlaybookFormat::Markdown,
        };

        let summary = playbook(args).expect("playbook renders");
        let text = summary.terminal_summary();

        assert!(text.contains("# CLIARE Harness Playbook"));
        assert!(text.contains("## 2. Read Agent Surface"));
        assert!(text.contains("cliare report harness --out .cliare/rote --format markdown"));
        assert!(text.contains("cliare skills install --agent all --scope project"));
        assert!(text.contains("AGENT_SKILL.md"));
    }

    #[test]
    fn security_playbook_contains_review_and_decision_loop() {
        let args = PlaybookArgs {
            role: PlaybookRole::Security,
            target: Some("rote".to_owned()),
            out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
            context: None,
            format: PlaybookFormat::Human,
        };

        let summary = playbook(args).expect("playbook renders");
        let text = summary.terminal_summary();

        assert!(text.contains("CLIARE security walkthrough"));
        assert!(text.contains("1. Measure Safely"));
        assert!(text.contains("cliare report security --out .cliare/rote --format markdown"));
        assert!(
            text.contains(
                "cliare issues mark <issue-id> --out .cliare/rote --status accepted-risk"
            )
        );
        assert!(text.contains("Completion criteria"));
    }
}
