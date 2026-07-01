use std::collections::BTreeSet;

use cliare_cli::cli::SurfaceOutputRequirement;

use super::index::{CommandIndexCommand, CommandIndexFlag, CommandIndexOutputContract};
use super::model::{SurfaceFlag, SurfaceOutputContract, SurfaceRequirement};
use super::tokens::{TokenSet, normalize_phrase};

pub(super) fn score_command(
    command: &CommandIndexCommand,
    intent_tokens: &TokenSet,
    require_output: Option<SurfaceOutputRequirement>,
) -> u32 {
    if intent_tokens.is_empty() {
        return 0;
    }

    let path_tokens = TokenSet::from_text(&command.path.join(" "));
    let summary_tokens = TokenSet::from_text(command.summary.as_deref().unwrap_or_default());
    let parameter_tokens = TokenSet::from_text(&parameter_text(command));
    let output_tokens = TokenSet::from_text(&output_text(command));
    let normalized_intent = normalize_phrase(
        &intent_tokens
            .tokens
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(" "),
    );
    let command_text = normalize_phrase(&format!(
        "{} {}",
        command.command,
        command.summary.as_deref().unwrap_or_default()
    ));

    let mut lexical_score = 0_u32;
    if !normalized_intent.is_empty() && command_text.contains(&normalized_intent) {
        lexical_score += 60;
    }

    for token in &intent_tokens.tokens {
        if path_tokens.contains(token) {
            lexical_score += 30;
        }
        if summary_tokens.contains(token) {
            lexical_score += 12;
        }
        if parameter_tokens.contains(token) {
            lexical_score += 8;
        }
        if output_tokens.contains(token) {
            lexical_score += 8;
        }
    }

    if lexical_score < 10 {
        return 0;
    }

    let output_bonus = require_output
        .filter(|requirement| best_output_contract(command, *requirement).is_some())
        .map(|_| 24)
        .unwrap_or(0);

    lexical_score + output_bonus + readiness_rank(&command.agent_suitability) * 3
}

pub(super) fn match_reason(
    command: &CommandIndexCommand,
    intent_tokens: &TokenSet,
    require_output: Option<SurfaceOutputRequirement>,
) -> String {
    let mut sources = Vec::new();
    let path_tokens = TokenSet::from_text(&command.path.join(" "));
    let summary_tokens = TokenSet::from_text(command.summary.as_deref().unwrap_or_default());
    let parameter_tokens = TokenSet::from_text(&parameter_text(command));
    let output_tokens = TokenSet::from_text(&output_text(command));

    if intent_tokens.intersects(&path_tokens) {
        sources.push("command path");
    }
    if intent_tokens.intersects(&summary_tokens) {
        sources.push("summary");
    }
    if intent_tokens.intersects(&parameter_tokens) {
        sources.push("parameters");
    }
    if intent_tokens.intersects(&output_tokens) {
        sources.push("output contracts");
    }
    if require_output.is_some() {
        sources.push("requested output capability");
    }
    sources.sort_unstable();
    sources.dedup();

    if sources.is_empty() {
        "Matched by readiness and measured command metadata.".to_owned()
    } else {
        format!("Matched intent using {}.", sources.join(", "))
    }
}

pub(super) fn output_requirement_matches(
    command: &CommandIndexCommand,
    require_output: Option<SurfaceOutputRequirement>,
) -> bool {
    require_output.is_none_or(|requirement| best_output_contract(command, requirement).is_some())
}

fn best_output_contract(
    command: &CommandIndexCommand,
    requirement: SurfaceOutputRequirement,
) -> Option<&CommandIndexOutputContract> {
    command
        .output_contracts
        .iter()
        .filter(|contract| output_requirement_accepts(requirement, &contract.mode))
        .max_by_key(|contract| output_status_rank(&contract.status))
}

fn output_requirement_accepts(requirement: SurfaceOutputRequirement, mode: &str) -> bool {
    match requirement {
        SurfaceOutputRequirement::Json => mode == "json",
        SurfaceOutputRequirement::Yaml => mode == "yaml" || mode == "yml",
        SurfaceOutputRequirement::MachineReadable => {
            matches!(mode, "json" | "yaml" | "yml")
        }
    }
}

fn output_status_rank(status: &str) -> u8 {
    match status {
        "parse_success" => 5,
        "unvalidated" => 4,
        "unprobed" => 3,
        "precondition_blocked" => 2,
        "help_text" => 1,
        "parse_failed" => 0,
        _ => 0,
    }
}

pub(super) fn readiness_rank(readiness: &str) -> u32 {
    match readiness {
        "ready" => 5,
        "conditional" => 4,
        "needs_fixture" => 3,
        "candidate" => 2,
        "blocked" => 1,
        _ => 0,
    }
}

fn parameter_text(command: &CommandIndexCommand) -> String {
    let mut text = String::new();
    for positional in &command.parameters.positionals {
        text.push(' ');
        text.push_str(&positional.name);
    }
    for flag in &command.parameters.flags {
        text.push(' ');
        text.push_str(&flag.name);
        if let Some(summary) = &flag.summary {
            text.push(' ');
            text.push_str(summary);
        }
        if let Some(value_name) = &flag.value_name {
            text.push(' ');
            text.push_str(value_name);
        }
    }
    text
}

fn output_text(command: &CommandIndexCommand) -> String {
    command
        .output_contracts
        .iter()
        .map(|contract| {
            format!(
                "{} {} {}",
                contract.mode,
                contract.status,
                contract.argv_fragment.join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn requirements(command: &CommandIndexCommand) -> Vec<SurfaceRequirement> {
    let mut requirements = Vec::new();
    for positional in &command.parameters.positionals {
        if positional.required {
            requirements.push(SurfaceRequirement {
                kind: "positional",
                name: positional.name.clone(),
                required: true,
                source: "usage",
            });
        }
    }
    for precondition in &command.preconditions {
        requirements.push(SurfaceRequirement {
            kind: "precondition",
            name: precondition.clone(),
            required: true,
            source: "command_index",
        });
    }
    requirements
}

pub(super) fn cautions(
    command: &CommandIndexCommand,
    output_contracts: &[SurfaceOutputContract],
    require_output: Option<SurfaceOutputRequirement>,
) -> Vec<String> {
    let mut cautions = Vec::new();
    match command.agent_suitability.as_str() {
        "blocked" => cautions.push("Command is blocked for automatic routing.".to_owned()),
        "candidate" => cautions.push("Command is inferred but not runtime-confirmed.".to_owned()),
        "needs_fixture" => {
            cautions.push("Command needs safe fixture data before routing.".to_owned())
        }
        _ => {}
    }
    for reason in &command.suitability_reasons {
        if reason != "runtime-confirmed command shape" {
            cautions.push(reason.clone());
        }
    }
    for contract in output_contracts {
        if contract.status != "parse_success" {
            cautions.push(format!(
                "{} output contract is {}.",
                contract.mode, contract.status
            ));
        }
    }
    if let Some(requirement) = require_output
        && best_output_contract(command, requirement).is_none()
    {
        cautions.push(format!(
            "Requested {} output is not advertised by this command.",
            requirement.label()
        ));
    }
    cautions.sort();
    cautions.dedup();
    cautions
}

pub(super) fn argv_template(
    command: &CommandIndexCommand,
    require_output: Option<SurfaceOutputRequirement>,
) -> Vec<String> {
    let mut argv = command.argv.clone();
    for positional in &command.parameters.positionals {
        if positional.required {
            let placeholder = if positional.variadic {
                format!("<{}>...", positional.name)
            } else {
                format!("<{}>", positional.name)
            };
            argv.push(placeholder);
        }
    }
    if let Some(requirement) = require_output
        && let Some(contract) = best_output_contract(command, requirement)
    {
        append_missing(&mut argv, &contract.argv_fragment);
    }
    argv
}

fn append_missing(argv: &mut Vec<String>, fragment: &[String]) {
    if fragment.is_empty() {
        return;
    }
    let exists = argv
        .windows(fragment.len())
        .any(|window| window == fragment);
    if !exists {
        argv.extend(fragment.iter().cloned());
    }
}

pub(super) fn suggested_flags(
    command: &CommandIndexCommand,
    intent_tokens: Option<&TokenSet>,
) -> Vec<SurfaceFlag> {
    let output_flag_names = command
        .output_contracts
        .iter()
        .map(|contract| contract.argv_fragment.first().unwrap_or(&contract.mode))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut flags = Vec::new();
    let mut seen = BTreeSet::new();

    for flag in &command.parameters.flags {
        let reason = suggested_flag_reason(flag, &output_flag_names, intent_tokens);
        if let Some(reason) = reason
            && seen.insert(flag.name.clone())
        {
            flags.push(SurfaceFlag {
                name: flag.name.clone(),
                short: flag.short.clone(),
                value_name: flag.value_name.clone(),
                summary: flag.summary.clone(),
                required: flag.required,
                repeatable: flag.repeatable,
                reason,
            });
        }
    }
    flags
}

fn suggested_flag_reason(
    flag: &CommandIndexFlag,
    output_flag_names: &BTreeSet<String>,
    intent_tokens: Option<&TokenSet>,
) -> Option<&'static str> {
    if flag.required {
        return Some("required");
    }
    if output_flag_names.contains(&flag.name) {
        return Some("output_mode");
    }
    match flag.name.as_str() {
        "--out" => Some("artifact_directory"),
        "--context" => Some("routing_context"),
        "--status" | "--reason" => Some("disposition"),
        _ => {
            let flag_tokens = TokenSet::from_text(&format!(
                "{} {} {}",
                flag.name,
                flag.summary.as_deref().unwrap_or_default(),
                flag.value_name.as_deref().unwrap_or_default()
            ));
            intent_tokens
                .filter(|tokens| tokens.intersects(&flag_tokens))
                .map(|_| "intent_match")
        }
    }
}

pub(super) fn use_when(readiness: &str) -> &'static str {
    match readiness {
        "ready" => "Use when the command matches the intent and local policy allows it.",
        "conditional" => {
            "Use when the listed cautions are acceptable or the harness can tolerate the condition."
        }
        "needs_fixture" => {
            "Do not route automatically until safe fixtures or operands are available."
        }
        "blocked" => "Do not route automatically until preconditions are provisioned.",
        "candidate" => "Do not route until runtime confirmation exists.",
        _ => "Review command-index.json before routing.",
    }
}
