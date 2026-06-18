use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::artifacts::COMMAND_INDEX_JSON;
use crate::cli::{
    SurfaceArgs, SurfaceCommand, SurfaceExplainArgs, SurfaceFormat, SurfaceListArgs,
    SurfaceOutputRequirement, SurfaceQueryArgs, SurfaceReadiness,
};
use crate::context;
use crate::error::{CliareError, Result};
use crate::report_format::shell_arg;

const SURFACE_QUERY_SCHEMA_VERSION: &str = "cliare.surface-query.v1";
const SURFACE_EXPLAIN_SCHEMA_VERSION: &str = "cliare.surface-explain.v1";
const SURFACE_LIST_SCHEMA_VERSION: &str = "cliare.surface-list.v1";

#[derive(Debug, Clone)]
pub struct SurfaceSummary {
    artifact_dir: PathBuf,
    rendered: String,
}

impl SurfaceSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.rendered
    }

    pub fn artifact_dir(&self) -> &Path {
        &self.artifact_dir
    }
}

pub async fn surface(args: SurfaceArgs) -> Result<SurfaceSummary> {
    match args.command {
        SurfaceCommand::Query(args) => query(args).await,
        SurfaceCommand::Explain(args) => explain(args).await,
        SurfaceCommand::List(args) => list(args).await,
    }
}

async fn query(args: SurfaceQueryArgs) -> Result<SurfaceSummary> {
    let artifact_dir = context::resolve_measurement_dir(
        &args.out,
        args.context.as_deref(),
        "cliare surface query",
    )
    .await?;
    let index = CommandIndexArtifact::read(&artifact_dir).await?;
    let packet = SurfaceQueryPacket::build(
        &artifact_dir,
        &args.intent,
        args.require_output,
        args.limit.min(20),
        &index,
    );
    let rendered = render_query_packet(&packet, args.format)?;

    Ok(SurfaceSummary {
        artifact_dir,
        rendered,
    })
}

async fn explain(args: SurfaceExplainArgs) -> Result<SurfaceSummary> {
    let artifact_dir = context::resolve_measurement_dir(
        &args.out,
        args.context.as_deref(),
        "cliare surface explain",
    )
    .await?;
    let index = CommandIndexArtifact::read(&artifact_dir).await?;
    let command_path = normalize_command_path(&args.command);
    let packet =
        SurfaceExplainPacket::build(&artifact_dir, command_path, args.require_output, &index);
    let rendered = render_explain_packet(&packet, args.format)?;

    Ok(SurfaceSummary {
        artifact_dir,
        rendered,
    })
}

async fn list(args: SurfaceListArgs) -> Result<SurfaceSummary> {
    let artifact_dir =
        context::resolve_measurement_dir(&args.out, args.context.as_deref(), "cliare surface list")
            .await?;
    let index = CommandIndexArtifact::read(&artifact_dir).await?;
    let packet = SurfaceListPacket::build(
        &artifact_dir,
        args.state,
        args.require_output,
        args.limit.min(200),
        &index,
    );
    let rendered = render_list_packet(&packet, args.format)?;

    Ok(SurfaceSummary {
        artifact_dir,
        rendered,
    })
}

#[derive(Debug, Serialize)]
struct SurfaceQueryPacket {
    schema_version: &'static str,
    artifact_dir: PathBuf,
    command_index: PathBuf,
    intent: String,
    require_output: Option<SurfaceOutputRequirement>,
    limit: usize,
    matches: Vec<SurfaceMatch>,
    no_match_reason: Option<String>,
}

impl SurfaceQueryPacket {
    fn build(
        artifact_dir: &Path,
        intent: &str,
        require_output: Option<SurfaceOutputRequirement>,
        limit: usize,
        index: &CommandIndexArtifact,
    ) -> Self {
        let intent_tokens = TokenSet::from_text(intent);
        let mut scored = index
            .commands
            .iter()
            .filter(|command| output_requirement_matches(command, require_output))
            .filter_map(|command| {
                let score = score_command(command, &intent_tokens, require_output);
                (score > 0).then_some((score, command))
            })
            .collect::<Vec<_>>();

        scored.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| {
                    readiness_rank(&right.agent_suitability)
                        .cmp(&readiness_rank(&left.agent_suitability))
                })
                .then_with(|| right.confidence.total_cmp(&left.confidence))
                .then_with(|| left.path.len().cmp(&right.path.len()))
                .then_with(|| left.path.cmp(&right.path))
        });

        let matches = scored
            .into_iter()
            .take(limit)
            .map(|(score, command)| {
                SurfaceMatch::from_command(
                    command,
                    Some(score),
                    require_output,
                    Some(&intent_tokens),
                    match_reason(command, &intent_tokens, require_output),
                )
            })
            .collect::<Vec<_>>();
        let no_match_reason = matches.is_empty().then(|| {
            if intent.trim().is_empty() {
                "Intent was empty; provide task words such as `check job status`.".to_owned()
            } else if require_output.is_some() {
                "No measured command matched the intent and requested output capability.".to_owned()
            } else {
                "No measured command matched the intent tokens.".to_owned()
            }
        });

        Self {
            schema_version: SURFACE_QUERY_SCHEMA_VERSION,
            artifact_dir: artifact_dir.to_path_buf(),
            command_index: artifact_dir.join(COMMAND_INDEX_JSON),
            intent: intent.to_owned(),
            require_output,
            limit,
            matches,
            no_match_reason,
        }
    }
}

#[derive(Debug, Serialize)]
struct SurfaceExplainPacket {
    schema_version: &'static str,
    artifact_dir: PathBuf,
    command_index: PathBuf,
    command: String,
    require_output: Option<SurfaceOutputRequirement>,
    surface: Option<SurfaceMatch>,
    no_match_reason: Option<String>,
}

impl SurfaceExplainPacket {
    fn build(
        artifact_dir: &Path,
        command_path: Vec<String>,
        require_output: Option<SurfaceOutputRequirement>,
        index: &CommandIndexArtifact,
    ) -> Self {
        let command = command_path.join(" ");
        let surface = index
            .commands
            .iter()
            .find(|candidate| candidate.path == command_path || candidate.command == command)
            .map(|candidate| {
                SurfaceMatch::from_command(
                    candidate,
                    None,
                    require_output,
                    None,
                    "Exact measured command path.".to_owned(),
                )
            });
        let no_match_reason = surface
            .is_none()
            .then(|| "No command with this path exists in command-index.json.".to_owned());

        Self {
            schema_version: SURFACE_EXPLAIN_SCHEMA_VERSION,
            artifact_dir: artifact_dir.to_path_buf(),
            command_index: artifact_dir.join(COMMAND_INDEX_JSON),
            command,
            require_output,
            surface,
            no_match_reason,
        }
    }
}

#[derive(Debug, Serialize)]
struct SurfaceListPacket {
    schema_version: &'static str,
    artifact_dir: PathBuf,
    command_index: PathBuf,
    state: Option<SurfaceReadiness>,
    require_output: Option<SurfaceOutputRequirement>,
    limit: usize,
    commands: Vec<SurfaceMatch>,
}

impl SurfaceListPacket {
    fn build(
        artifact_dir: &Path,
        state: Option<SurfaceReadiness>,
        require_output: Option<SurfaceOutputRequirement>,
        limit: usize,
        index: &CommandIndexArtifact,
    ) -> Self {
        let mut commands = index
            .commands
            .iter()
            .filter(|command| state.is_none_or(|state| command.agent_suitability == state.label()))
            .filter(|command| output_requirement_matches(command, require_output))
            .collect::<Vec<_>>();
        commands.sort_by(|left, right| {
            readiness_rank(&right.agent_suitability)
                .cmp(&readiness_rank(&left.agent_suitability))
                .then_with(|| right.confidence.total_cmp(&left.confidence))
                .then_with(|| left.path.len().cmp(&right.path.len()))
                .then_with(|| left.path.cmp(&right.path))
        });

        Self {
            schema_version: SURFACE_LIST_SCHEMA_VERSION,
            artifact_dir: artifact_dir.to_path_buf(),
            command_index: artifact_dir.join(COMMAND_INDEX_JSON),
            state,
            require_output,
            limit,
            commands: commands
                .into_iter()
                .take(limit)
                .map(|command| {
                    SurfaceMatch::from_command(
                        command,
                        None,
                        require_output,
                        None,
                        "Listed from measured command index.".to_owned(),
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SurfaceMatch {
    id: String,
    command: String,
    path: Vec<String>,
    summary: Option<String>,
    readiness: String,
    runtime_state: String,
    confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    match_score: Option<u32>,
    argv_template: Vec<String>,
    required_positionals: Vec<SurfacePositional>,
    suggested_flags: Vec<SurfaceFlag>,
    requires: Vec<SurfaceRequirement>,
    output_contracts: Vec<SurfaceOutputContract>,
    cautions: Vec<String>,
    gaps: Vec<SurfaceGap>,
    evidence: Vec<String>,
    why: String,
    use_when: String,
}

impl SurfaceMatch {
    fn from_command(
        command: &CommandIndexCommand,
        match_score: Option<u32>,
        require_output: Option<SurfaceOutputRequirement>,
        intent_tokens: Option<&TokenSet>,
        why: String,
    ) -> Self {
        let output_contracts = command
            .output_contracts
            .iter()
            .map(SurfaceOutputContract::from_contract)
            .collect::<Vec<_>>();
        let required_positionals = command
            .parameters
            .positionals
            .iter()
            .filter(|positional| positional.required)
            .map(SurfacePositional::from_positional)
            .collect::<Vec<_>>();
        let suggested_flags = suggested_flags(command, intent_tokens);
        let requires = requirements(command);
        let cautions = cautions(command, &output_contracts, require_output);

        Self {
            id: command.id.clone(),
            command: command.command.clone(),
            path: command.path.clone(),
            summary: command.summary.clone(),
            readiness: command.agent_suitability.clone(),
            runtime_state: command.runtime_state.clone(),
            confidence: command.confidence,
            match_score,
            argv_template: argv_template(command, require_output),
            required_positionals,
            suggested_flags,
            requires,
            output_contracts,
            cautions,
            gaps: command.gaps.iter().map(SurfaceGap::from_gap).collect(),
            evidence: command.evidence.iter().take(8).cloned().collect(),
            why,
            use_when: use_when(&command.agent_suitability).to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SurfaceRequirement {
    kind: &'static str,
    name: String,
    required: bool,
    source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct SurfacePositional {
    name: String,
    required: bool,
    variadic: bool,
}

impl SurfacePositional {
    fn from_positional(positional: &CommandIndexPositional) -> Self {
        Self {
            name: positional.name.clone(),
            required: positional.required,
            variadic: positional.variadic,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SurfaceFlag {
    name: String,
    short: Option<String>,
    value_name: Option<String>,
    summary: Option<String>,
    required: bool,
    repeatable: bool,
    reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct SurfaceOutputContract {
    mode: String,
    status: String,
    argv_fragment: Vec<String>,
    observed_kind: Option<String>,
    preconditions: Vec<String>,
    diagnostic: Option<String>,
}

impl SurfaceOutputContract {
    fn from_contract(contract: &CommandIndexOutputContract) -> Self {
        Self {
            mode: contract.mode.clone(),
            status: contract.status.clone(),
            argv_fragment: contract.argv_fragment.clone(),
            observed_kind: contract.observed_kind.clone(),
            preconditions: contract.preconditions.clone(),
            diagnostic: contract.diagnostic.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SurfaceGap {
    kind: String,
    reason: String,
}

impl SurfaceGap {
    fn from_gap(gap: &CommandIndexGap) -> Self {
        Self {
            kind: gap.kind.clone(),
            reason: gap.reason.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CommandIndexArtifact {
    commands: Vec<CommandIndexCommand>,
}

impl CommandIndexArtifact {
    async fn read(artifact_dir: &Path) -> Result<Self> {
        let path = artifact_dir.join(COMMAND_INDEX_JSON);
        let bytes = fs::read(&path)
            .await
            .map_err(|source| CliareError::ReadCommandIndex {
                path: path.clone(),
                source,
            })?;
        serde_json::from_slice(&bytes)
            .map_err(|source| CliareError::ParseCommandIndex { path, source })
    }
}

#[derive(Debug, Deserialize)]
struct CommandIndexCommand {
    id: String,
    command: String,
    path: Vec<String>,
    argv: Vec<String>,
    summary: Option<String>,
    runtime_state: String,
    agent_suitability: String,
    #[serde(default)]
    suitability_reasons: Vec<String>,
    #[serde(default)]
    confidence: f64,
    parameters: CommandIndexParameters,
    #[serde(default)]
    preconditions: Vec<String>,
    #[serde(default)]
    output_contracts: Vec<CommandIndexOutputContract>,
    #[serde(default)]
    gaps: Vec<CommandIndexGap>,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CommandIndexParameters {
    #[serde(default)]
    positionals: Vec<CommandIndexPositional>,
    #[serde(default)]
    flags: Vec<CommandIndexFlag>,
}

#[derive(Debug, Deserialize)]
struct CommandIndexPositional {
    name: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    variadic: bool,
}

#[derive(Debug, Deserialize)]
struct CommandIndexFlag {
    name: String,
    short: Option<String>,
    summary: Option<String>,
    value_name: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    repeatable: bool,
}

#[derive(Debug, Deserialize)]
struct CommandIndexOutputContract {
    mode: String,
    #[serde(default)]
    argv_fragment: Vec<String>,
    status: String,
    #[serde(default)]
    preconditions: Vec<String>,
    observed_kind: Option<String>,
    diagnostic: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommandIndexGap {
    kind: String,
    reason: String,
}

#[derive(Debug)]
struct TokenSet {
    tokens: BTreeSet<String>,
}

impl TokenSet {
    fn from_text(text: &str) -> Self {
        let mut tokens = BTreeSet::new();
        for token in tokenize(text) {
            tokens.insert(token);
        }
        Self { tokens }
    }

    fn contains(&self, token: &str) -> bool {
        let needle_variants = token_variants(token);
        needle_variants
            .iter()
            .any(|variant| self.tokens.contains(variant))
            || self.tokens.iter().any(|own_token| {
                token_variants(own_token)
                    .iter()
                    .any(|variant| variant == token)
            })
    }

    fn intersects(&self, other: &Self) -> bool {
        self.tokens.iter().any(|token| other.contains(token))
    }

    fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

fn score_command(
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

fn match_reason(
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

fn output_requirement_matches(
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

fn readiness_rank(readiness: &str) -> u32 {
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

fn requirements(command: &CommandIndexCommand) -> Vec<SurfaceRequirement> {
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

fn cautions(
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

fn argv_template(
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

fn suggested_flags(
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

fn use_when(readiness: &str) -> &'static str {
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

fn render_query_packet(packet: &SurfaceQueryPacket, format: SurfaceFormat) -> Result<String> {
    match format {
        SurfaceFormat::Json => serialize_surface(packet),
        SurfaceFormat::Human => {
            let mut text = String::new();
            writeln!(text, "Surface query: {}", packet.intent)
                .expect("writing to string cannot fail");
            writeln!(text, "Artifact: {}", packet.artifact_dir.display())
                .expect("writing to string cannot fail");
            if let Some(requirement) = packet.require_output {
                writeln!(text, "Required output: {}", requirement.label())
                    .expect("writing to string cannot fail");
            }
            render_matches(
                &mut text,
                &packet.matches,
                packet.no_match_reason.as_deref(),
            );
            Ok(text)
        }
    }
}

fn render_explain_packet(packet: &SurfaceExplainPacket, format: SurfaceFormat) -> Result<String> {
    match format {
        SurfaceFormat::Json => serialize_surface(packet),
        SurfaceFormat::Human => {
            let mut text = String::new();
            writeln!(text, "Surface explain: {}", packet.command)
                .expect("writing to string cannot fail");
            writeln!(text, "Artifact: {}", packet.artifact_dir.display())
                .expect("writing to string cannot fail");
            match &packet.surface {
                Some(surface) => render_match(&mut text, 1, surface),
                None => {
                    writeln!(
                        text,
                        "{}",
                        packet
                            .no_match_reason
                            .as_deref()
                            .unwrap_or("No command matched.")
                    )
                    .expect("writing to string cannot fail");
                }
            }
            Ok(text)
        }
    }
}

fn render_list_packet(packet: &SurfaceListPacket, format: SurfaceFormat) -> Result<String> {
    match format {
        SurfaceFormat::Json => serialize_surface(packet),
        SurfaceFormat::Human => {
            let mut text = String::new();
            writeln!(text, "Surface list").expect("writing to string cannot fail");
            writeln!(text, "Artifact: {}", packet.artifact_dir.display())
                .expect("writing to string cannot fail");
            if let Some(state) = packet.state {
                writeln!(text, "Readiness: {}", state.label())
                    .expect("writing to string cannot fail");
            }
            if let Some(requirement) = packet.require_output {
                writeln!(text, "Required output: {}", requirement.label())
                    .expect("writing to string cannot fail");
            }
            render_matches(&mut text, &packet.commands, None);
            Ok(text)
        }
    }
}

fn render_matches(text: &mut String, matches: &[SurfaceMatch], no_match_reason: Option<&str>) {
    if matches.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(
            text,
            "{}",
            no_match_reason.unwrap_or("No commands matched.")
        )
        .expect("writing to string cannot fail");
        return;
    }
    for (index, surface) in matches.iter().enumerate() {
        render_match(text, index + 1, surface);
    }
}

fn render_match(text: &mut String, index: usize, surface: &SurfaceMatch) {
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "{}. {} [{}]",
        index, surface.command, surface.readiness
    )
    .expect("writing to string cannot fail");
    if let Some(summary) = &surface.summary {
        writeln!(text, "   {}", summary).expect("writing to string cannot fail");
    }
    writeln!(
        text,
        "   invoke: {}",
        surface
            .argv_template
            .iter()
            .map(|arg| shell_arg(arg))
            .collect::<Vec<_>>()
            .join(" ")
    )
    .expect("writing to string cannot fail");
    writeln!(text, "   why: {}", surface.why).expect("writing to string cannot fail");
    if !surface.requires.is_empty() {
        writeln!(
            text,
            "   requires: {}",
            surface
                .requires
                .iter()
                .map(|requirement| requirement.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("writing to string cannot fail");
    }
    if !surface.cautions.is_empty() {
        writeln!(text, "   cautions: {}", surface.cautions.join("; "))
            .expect("writing to string cannot fail");
    }
}

fn serialize_surface<T: Serialize>(packet: &T) -> Result<String> {
    serde_json::to_string_pretty(packet)
        .map(|json| json + "\n")
        .map_err(CliareError::SerializeSurface)
}

fn normalize_command_path(raw: &[String]) -> Vec<String> {
    if raw.len() == 1 {
        raw[0].split_whitespace().map(ToOwned::to_owned).collect()
    } else {
        raw.to_vec()
    }
}

fn normalize_phrase(text: &str) -> String {
    tokenize(text).join(" ")
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn token_variants(token: &str) -> Vec<String> {
    let mut variants = vec![token.to_owned()];
    if token.len() > 3 && token.ends_with('s') && !token.ends_with("ss") && !token.ends_with("us") {
        variants.push(token.trim_end_matches('s').to_owned());
    }
    variants
}

#[cfg(test)]
mod tests {
    use super::{
        CommandIndexArtifact, CommandIndexCommand, CommandIndexFlag, CommandIndexGap,
        CommandIndexOutputContract, CommandIndexParameters, CommandIndexPositional,
        SurfaceExplainPacket, SurfaceListPacket, SurfaceOutputRequirement, SurfaceQueryPacket,
        SurfaceReadiness,
    };
    use std::path::Path;

    #[test]
    fn query_matches_intent_to_command_and_synthesizes_template() {
        let index = test_index();
        let packet = SurfaceQueryPacket::build(
            Path::new(".cliare/test"),
            "check job status",
            None,
            3,
            &index,
        );

        let first = packet.matches.first().expect("query has a match");
        assert_eq!(first.command, "jobs status");
        assert_eq!(first.readiness, "conditional");
        assert_eq!(first.argv_template, ["cliare", "jobs", "status"]);
        assert!(first.why.contains("command path"));
        assert!(
            first
                .suggested_flags
                .iter()
                .any(|flag| flag.name == "--out" && flag.reason == "artifact_directory")
        );
    }

    #[test]
    fn query_can_require_json_output() {
        let index = test_index();
        let packet = SurfaceQueryPacket::build(
            Path::new(".cliare/test"),
            "list issues",
            Some(SurfaceOutputRequirement::Json),
            3,
            &index,
        );

        let first = packet.matches.first().expect("query has a JSON match");
        assert_eq!(first.command, "issues list");
        assert_eq!(
            first.argv_template,
            ["cliare", "issues", "list", "--format", "json"]
        );
        assert!(
            first
                .cautions
                .iter()
                .any(|caution| caution.contains("json output contract is unprobed"))
        );
    }

    #[test]
    fn explain_reports_required_positionals_and_disposition_flags() {
        let index = test_index();
        let packet = SurfaceExplainPacket::build(
            Path::new(".cliare/test"),
            vec!["issues".to_owned(), "mark".to_owned()],
            None,
            &index,
        );

        let surface = packet.surface.expect("command exists");
        assert_eq!(surface.command, "issues mark");
        assert_eq!(
            surface.argv_template,
            ["cliare", "issues", "mark", "<issue_id>"]
        );
        assert_eq!(surface.required_positionals[0].name, "issue_id");
        assert!(
            surface
                .suggested_flags
                .iter()
                .any(|flag| flag.name == "--status" && flag.reason == "disposition")
        );
    }

    #[test]
    fn list_filters_by_readiness() {
        let index = test_index();
        let packet = SurfaceListPacket::build(
            Path::new(".cliare/test"),
            Some(SurfaceReadiness::Conditional),
            None,
            10,
            &index,
        );

        assert!(
            packet
                .commands
                .iter()
                .all(|command| command.readiness == "conditional")
        );
        assert!(
            packet
                .commands
                .iter()
                .any(|command| command.command == "jobs status")
        );
    }

    fn test_index() -> CommandIndexArtifact {
        CommandIndexArtifact {
            commands: vec![
                CommandIndexCommand {
                    id: "cliare.jobs.status".to_owned(),
                    command: "jobs status".to_owned(),
                    path: vec!["jobs".to_owned(), "status".to_owned()],
                    argv: vec!["cliare".to_owned(), "jobs".to_owned(), "status".to_owned()],
                    summary: Some(
                        "Print the latest detached or foreground measurement progress state"
                            .to_owned(),
                    ),
                    runtime_state: "runtime_confirmed".to_owned(),
                    agent_suitability: "conditional".to_owned(),
                    suitability_reasons: vec![
                        "invalid_flag_diagnostics_unknown: safe invalid-flag probe has not observed flag diagnostics"
                            .to_owned(),
                    ],
                    confidence: 0.99,
                    parameters: CommandIndexParameters {
                        positionals: Vec::new(),
                        flags: vec![CommandIndexFlag {
                            name: "--out".to_owned(),
                            short: None,
                            summary: Some(
                                "Measurement artifact directory containing jobs/current".to_owned(),
                            ),
                            value_name: Some("dir".to_owned()),
                            required: false,
                            repeatable: false,
                        }],
                    },
                    preconditions: Vec::new(),
                    output_contracts: Vec::new(),
                    gaps: vec![CommandIndexGap {
                        kind: "invalid_flag_diagnostics_unknown".to_owned(),
                        reason: "safe invalid-flag probe has not observed flag diagnostics"
                            .to_owned(),
                    }],
                    evidence: vec!["e_0001".to_owned()],
                },
                CommandIndexCommand {
                    id: "cliare.issues.list".to_owned(),
                    command: "issues list".to_owned(),
                    path: vec!["issues".to_owned(), "list".to_owned()],
                    argv: vec!["cliare".to_owned(), "issues".to_owned(), "list".to_owned()],
                    summary: Some("List generated issues with maintainer dispositions".to_owned()),
                    runtime_state: "runtime_confirmed".to_owned(),
                    agent_suitability: "needs_fixture".to_owned(),
                    suitability_reasons: vec![
                        "machine-readable output contract needs fixture or command-local validation"
                            .to_owned(),
                    ],
                    confidence: 0.98,
                    parameters: CommandIndexParameters {
                        positionals: Vec::new(),
                        flags: vec![
                            CommandIndexFlag {
                                name: "--format".to_owned(),
                                short: None,
                                summary: Some(
                                    "Output format [possible values: human, markdown, json]"
                                        .to_owned(),
                                ),
                                value_name: Some("format".to_owned()),
                                required: false,
                                repeatable: false,
                            },
                            CommandIndexFlag {
                                name: "--out".to_owned(),
                                short: None,
                                summary: Some(
                                    "Measurement artifact directory containing issues.json"
                                        .to_owned(),
                                ),
                                value_name: Some("dir".to_owned()),
                                required: false,
                                repeatable: false,
                            },
                        ],
                    },
                    preconditions: Vec::new(),
                    output_contracts: vec![CommandIndexOutputContract {
                        mode: "json".to_owned(),
                        argv_fragment: vec!["--format".to_owned(), "json".to_owned()],
                        status: "unprobed".to_owned(),
                        preconditions: Vec::new(),
                        observed_kind: None,
                        diagnostic: None,
                    }],
                    gaps: Vec::new(),
                    evidence: vec!["e_0002".to_owned()],
                },
                CommandIndexCommand {
                    id: "cliare.issues.mark".to_owned(),
                    command: "issues mark".to_owned(),
                    path: vec!["issues".to_owned(), "mark".to_owned()],
                    argv: vec!["cliare".to_owned(), "issues".to_owned(), "mark".to_owned()],
                    summary: Some("Record a maintainer disposition for an issue id".to_owned()),
                    runtime_state: "runtime_confirmed".to_owned(),
                    agent_suitability: "conditional".to_owned(),
                    suitability_reasons: Vec::new(),
                    confidence: 0.98,
                    parameters: CommandIndexParameters {
                        positionals: vec![CommandIndexPositional {
                            name: "issue_id".to_owned(),
                            required: true,
                            variadic: false,
                        }],
                        flags: vec![
                            CommandIndexFlag {
                                name: "--status".to_owned(),
                                short: None,
                                summary: Some("Maintainer disposition to record".to_owned()),
                                value_name: Some("status".to_owned()),
                                required: false,
                                repeatable: false,
                            },
                            CommandIndexFlag {
                                name: "--reason".to_owned(),
                                short: None,
                                summary: Some("Maintainer rationale for the disposition".to_owned()),
                                value_name: Some("reason".to_owned()),
                                required: false,
                                repeatable: false,
                            },
                        ],
                    },
                    preconditions: Vec::new(),
                    output_contracts: Vec::new(),
                    gaps: Vec::new(),
                    evidence: vec!["e_0003".to_owned()],
                },
            ],
        }
    }
}
