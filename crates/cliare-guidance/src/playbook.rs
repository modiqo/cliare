use std::path::PathBuf;

use serde::Serialize;

use cliare_cli::cli::{PlaybookArgs, PlaybookFormat, PlaybookRole};
use cliare_core::error::Result;

mod commands;
mod data;
mod lifecycles;
mod render;
#[cfg(test)]
mod tests;

use commands::{CommandBuilder, effective_artifact_dir};
use data::{artifact_layout, parameter_guide};
use lifecycles::{harness_lifecycle, maintainer_lifecycle, security_lifecycle};

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
        PlaybookFormat::Human => render::render_human(&packet),
        PlaybookFormat::Markdown => render::render_markdown(&packet),
        PlaybookFormat::Json => format!(
            "{}
",
            serde_json::to_string_pretty(&packet)
                .map_err(cliare_core::error::CliareError::SerializePlaybook)?
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
            goal: "Consume CLIARE artifacts as an agent-routing contract: surface resolver first, command index as audit evidence, harness packet second, generated skill third, then validate agent behavior against evidence.",
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
                "Agent routing starts from `cliare surface query` or `cliare surface explain`, with `command-index.json` available as audit evidence.",
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
