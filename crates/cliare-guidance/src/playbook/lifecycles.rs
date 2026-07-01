use super::ISSUE_PLACEHOLDER;
use super::commands::CommandBuilder;
use super::{PlaybookCommand, PlaybookSection};

pub(super) fn maintainer_lifecycle(commands: &CommandBuilder<'_>) -> Vec<PlaybookSection> {
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

pub(super) fn harness_lifecycle(commands: &CommandBuilder<'_>) -> Vec<PlaybookSection> {
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
            purpose: "Use the surface resolver for intent-to-command routing, with the command index and harness packet as evidence.",
            commands: vec![
                PlaybookCommand {
                    title: "Resolve an intent",
                    command: commands.surface_query("check job status", &["--format", "json"]),
                    why: "Returns ranked command matches, argv templates, readiness, requirements, and cautions without loading the full command index into the harness.",
                },
                PlaybookCommand {
                    title: "Explain a command",
                    command: commands.surface_explain("jobs status", &["--format", "json"]),
                    why: "Shows the measured invocation shape, required operands, suggested flags, output contracts, gaps, and evidence for one command.",
                },
                PlaybookCommand {
                    title: "List routable commands",
                    command: commands.surface_list(&["--state", "ready", "--format", "json"]),
                    why: "Gives harness code a compact readiness-filtered projection instead of the full command-index artifact.",
                },
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

pub(super) fn security_lifecycle(commands: &CommandBuilder<'_>) -> Vec<PlaybookSection> {
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
