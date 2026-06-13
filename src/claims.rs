use std::collections::{BTreeMap, BTreeSet};

use crate::belief::Belief;
use crate::evidence::{ProbeIntent, ProcessStatus};
use crate::layout;
use crate::observation::ShapeObservation;
use crate::output::{self, ObservedOutputKind, OutputMode};
use crate::precondition::{self, PreconditionKind};

const COMMAND_PRIOR: f64 = 0.08;
const FLAG_PRIOR: f64 = 0.12;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandPath(Vec<String>);

impl CommandPath {
    pub fn new(path: Vec<String>) -> Self {
        Self(path)
    }

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.0.clone()
    }
}

#[derive(Debug)]
pub struct ClaimSet {
    commands: BTreeMap<CommandPath, CommandClaim>,
    flags: BTreeMap<(CommandPath, String), FlagClaim>,
    outputs: BTreeMap<OutputContractKey, OutputContractClaim>,
}

impl ClaimSet {
    pub fn from_observations(binary_name: &str, observations: &[ShapeObservation]) -> Self {
        let mut claims = Self {
            commands: BTreeMap::new(),
            flags: BTreeMap::new(),
            outputs: BTreeMap::new(),
        };

        for observation in observations {
            claims.apply_observation(binary_name, observation);
        }

        claims
    }

    pub fn commands(&self) -> impl Iterator<Item = &CommandClaim> {
        self.commands.values()
    }

    pub fn flags(&self) -> impl Iterator<Item = &FlagClaim> {
        self.flags.values()
    }

    pub fn output_contracts(&self) -> impl Iterator<Item = &OutputContractClaim> {
        self.outputs.values()
    }

    fn apply_observation(&mut self, binary_name: &str, observation: &ShapeObservation) {
        match observation.intent {
            ProbeIntent::Help => self.apply_help_observation(binary_name, observation),
            ProbeIntent::InvalidChild => self.apply_invalid_child_observation(observation),
            ProbeIntent::InvalidFlag => self.apply_invalid_flag_observation(observation),
            ProbeIntent::OutputJson
            | ProbeIntent::OutputYaml
            | ProbeIntent::OutputTable
            | ProbeIntent::OutputPlain => self.apply_output_mode_observation(observation),
            ProbeIntent::Version | ProbeIntent::InvalidCommand => {}
        }
    }

    fn apply_help_observation(&mut self, binary_name: &str, observation: &ShapeObservation) {
        let help_like = successful_help_like(observation);
        let precondition = precondition_for_observation(observation);
        let current_path = CommandPath::new(observation.path.clone());

        if !current_path.is_empty() {
            self.command_mut(current_path.clone()).apply_runtime_help(
                &observation.evidence_id,
                help_like,
                precondition,
            );
        }

        let Some(text) = observation.process.stdout.text.as_deref() else {
            return;
        };
        if !help_like && !current_path.is_empty() {
            return;
        }

        if !current_path.is_empty() {
            let arguments = layout::usage_arguments(text, binary_name, current_path.as_slice());
            self.command_mut(current_path.clone())
                .apply_usage_arguments(arguments, &observation.evidence_id);
        }

        if current_path.is_empty() || !layout::is_manpage_like(text) {
            for candidate in layout::command_candidates(text, binary_name) {
                let path = CommandPath::new(absolutize_candidate_path(
                    current_path.as_slice(),
                    candidate.path,
                ));
                if path == current_path {
                    continue;
                }
                if is_child_path(current_path.as_slice(), path.as_slice()) {
                    self.command_mut(current_path.clone())
                        .apply_child_candidate(&observation.evidence_id);
                }

                self.command_mut(path).apply_layout_candidate(
                    candidate.summary,
                    candidate.aliases,
                    &observation.evidence_id,
                    &candidate.evidence_detail,
                );
            }
        }

        for candidate in layout::flag_candidates(text) {
            let key = (current_path.clone(), candidate.name.clone());
            self.flags
                .entry(key)
                .or_insert_with(|| FlagClaim::new(current_path.clone(), candidate.name.clone()))
                .apply_layout_candidate(candidate, &observation.evidence_id);
        }

        for candidate in layout::output_mode_candidates(text) {
            let key = OutputContractKey {
                command_path: current_path.clone(),
                mode: candidate.mode,
                argv_fragment: candidate.argv_fragment.clone(),
            };
            self.outputs
                .entry(key)
                .or_insert_with(|| {
                    OutputContractClaim::new(
                        current_path.clone(),
                        candidate.mode,
                        candidate.flag_name.clone(),
                        candidate.argv_fragment.clone(),
                    )
                })
                .apply_advertised(&observation.evidence_id, &candidate.evidence_detail);
        }
    }

    fn apply_invalid_child_observation(&mut self, observation: &ShapeObservation) {
        let path = CommandPath::new(observation.path.clone());
        if path.is_empty() {
            return;
        }
        if let Some(precondition) = precondition_for_observation(observation) {
            self.command_mut(path)
                .apply_precondition_blocked(&observation.evidence_id, precondition);
            return;
        }

        self.command_mut(path)
            .apply_invalid_child(&observation.evidence_id, rejected_by_runtime(observation));
    }

    fn apply_invalid_flag_observation(&mut self, observation: &ShapeObservation) {
        let path = CommandPath::new(observation.path.clone());
        if path.is_empty() {
            return;
        }
        if let Some(precondition) = precondition_for_observation(observation) {
            self.command_mut(path)
                .apply_precondition_blocked(&observation.evidence_id, precondition);
            return;
        }

        self.command_mut(path)
            .apply_invalid_flag(&observation.evidence_id, rejected_by_runtime(observation));
    }

    fn apply_output_mode_observation(&mut self, observation: &ShapeObservation) {
        let Some(mode) = output_mode_for_intent(observation.intent) else {
            return;
        };
        let path = CommandPath::new(observation.path.clone());
        let precondition = precondition_for_observation(observation);
        if let Some(precondition) = precondition {
            self.command_mut(path.clone())
                .apply_precondition_blocked(&observation.evidence_id, precondition);
        }
        let argv_fragment = output_probe_fragment(observation, path.as_slice());

        let matching_keys = self
            .outputs
            .keys()
            .filter(|key| {
                key.command_path == path
                    && key.mode == mode
                    && argv_contains_fragment(&argv_fragment, &key.argv_fragment)
            })
            .cloned()
            .collect::<Vec<_>>();

        if matching_keys.is_empty() {
            let key = OutputContractKey {
                command_path: path.clone(),
                mode,
                argv_fragment: argv_fragment.clone(),
            };
            let claim = self.outputs.entry(key).or_insert_with(|| {
                OutputContractClaim::new(path, mode, mode.label().to_owned(), argv_fragment)
            });
            if let Some(precondition) = precondition {
                claim.apply_precondition_probe(&observation.evidence_id, precondition);
            } else {
                claim.apply_probe(
                    &observation.evidence_id,
                    output::classify(
                        mode,
                        &observation.process.status,
                        observation.process.stdout.text.as_deref(),
                    ),
                );
            }
            return;
        }

        for key in matching_keys {
            if let Some(claim) = self.outputs.get_mut(&key) {
                if let Some(precondition) = precondition {
                    claim.apply_precondition_probe(&observation.evidence_id, precondition);
                } else {
                    claim.apply_probe(
                        &observation.evidence_id,
                        output::classify(
                            mode,
                            &observation.process.status,
                            observation.process.stdout.text.as_deref(),
                        ),
                    );
                }
            }
        }
    }

    fn command_mut(&mut self, path: CommandPath) -> &mut CommandClaim {
        self.commands
            .entry(path.clone())
            .or_insert_with(|| CommandClaim::new(path))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct OutputContractKey {
    command_path: CommandPath,
    mode: OutputMode,
    argv_fragment: Vec<String>,
}

#[derive(Debug)]
pub struct CommandClaim {
    path: CommandPath,
    summary: Option<String>,
    aliases: BTreeSet<String>,
    positionals: BTreeMap<String, PositionalClaim>,
    usage_observed: bool,
    belief: Belief,
    runtime_confirmed: bool,
    help_unavailable: bool,
    preconditions: BTreeSet<PreconditionKind>,
    has_child_candidates: bool,
    invalid_child_rejected: bool,
    invalid_flag_rejected: bool,
    evidence: Vec<String>,
}

impl CommandClaim {
    fn new(path: CommandPath) -> Self {
        Self {
            path,
            summary: None,
            aliases: BTreeSet::new(),
            positionals: BTreeMap::new(),
            usage_observed: false,
            belief: Belief::with_prior(COMMAND_PRIOR),
            runtime_confirmed: false,
            help_unavailable: false,
            preconditions: BTreeSet::new(),
            has_child_candidates: false,
            invalid_child_rejected: false,
            invalid_flag_rejected: false,
            evidence: Vec::new(),
        }
    }

    fn apply_layout_candidate(
        &mut self,
        summary: Option<String>,
        aliases: Vec<String>,
        evidence_id: &str,
        evidence_detail: &str,
    ) {
        self.belief.update(1.0);
        if self.summary.is_none() {
            self.summary = summary;
        }
        self.aliases.extend(aliases);
        self.evidence
            .push(format!("{evidence_id}:{evidence_detail}"));
    }

    fn apply_usage_arguments(
        &mut self,
        arguments: Vec<layout::CandidateArgument>,
        evidence_id: &str,
    ) {
        self.usage_observed = true;
        self.belief.update(0.5);
        for argument in arguments {
            self.positionals
                .entry(argument.name.clone())
                .and_modify(|existing| {
                    existing.required |= argument.required;
                    existing.variadic |= argument.variadic;
                    existing
                        .evidence
                        .push(format!("{evidence_id}:{}", argument.evidence_detail));
                })
                .or_insert_with(|| PositionalClaim {
                    name: argument.name,
                    required: argument.required,
                    variadic: argument.variadic,
                    evidence: vec![format!("{evidence_id}:{}", argument.evidence_detail)],
                });
        }
        self.evidence.push(format!("{evidence_id}:usage syntax"));
    }

    fn apply_runtime_help(
        &mut self,
        evidence_id: &str,
        help_like: bool,
        precondition: Option<PreconditionKind>,
    ) {
        if help_like {
            self.belief.update(4.0);
            self.runtime_confirmed = true;
        } else if let Some(precondition) = precondition {
            self.apply_precondition_blocked(evidence_id, precondition);
            return;
        } else {
            self.belief.update(-2.0);
            self.help_unavailable = true;
        }
        self.evidence.push(evidence_id.to_owned());
    }

    fn apply_precondition_blocked(&mut self, evidence_id: &str, precondition: PreconditionKind) {
        self.belief.update(2.0);
        self.preconditions.insert(precondition);
        self.evidence
            .push(format!("{evidence_id}:{}", precondition.label()));
    }

    fn apply_child_candidate(&mut self, evidence_id: &str) {
        self.has_child_candidates = true;
        self.evidence
            .push(format!("{evidence_id}:child command row"));
    }

    fn apply_invalid_child(&mut self, evidence_id: &str, rejected: bool) {
        if rejected {
            self.invalid_child_rejected = true;
            self.belief.update(0.5);
        }
        self.evidence.push(evidence_id.to_owned());
    }

    fn apply_invalid_flag(&mut self, evidence_id: &str, rejected: bool) {
        if rejected {
            self.invalid_flag_rejected = true;
            self.belief.update(0.5);
        }
        self.evidence.push(evidence_id.to_owned());
    }

    pub fn path(&self) -> &CommandPath {
        &self.path
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    pub fn aliases(&self) -> impl Iterator<Item = &String> {
        self.aliases.iter()
    }

    pub fn positionals(&self) -> impl Iterator<Item = &PositionalClaim> {
        self.positionals.values()
    }

    pub fn usage_observed(&self) -> bool {
        self.usage_observed
    }

    pub fn confidence(&self) -> f64 {
        self.belief.probability()
    }

    pub fn runtime_confirmed(&self) -> bool {
        self.runtime_confirmed
    }

    pub fn help_unavailable(&self) -> bool {
        self.help_unavailable
    }

    pub fn precondition_blocked(&self) -> bool {
        !self.preconditions.is_empty()
    }

    pub fn preconditions(&self) -> impl Iterator<Item = PreconditionKind> + '_ {
        self.preconditions.iter().copied()
    }

    pub fn has_child_candidates(&self) -> bool {
        self.has_child_candidates
    }

    pub fn invalid_child_rejected(&self) -> bool {
        self.invalid_child_rejected
    }

    pub fn invalid_flag_rejected(&self) -> bool {
        self.invalid_flag_rejected
    }

    pub fn evidence(&self) -> &[String] {
        &self.evidence
    }
}

#[derive(Debug, Clone)]
pub struct PositionalClaim {
    name: String,
    required: bool,
    variadic: bool,
    evidence: Vec<String>,
}

impl PositionalClaim {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn required(&self) -> bool {
        self.required
    }

    pub fn variadic(&self) -> bool {
        self.variadic
    }

    pub fn evidence(&self) -> &[String] {
        &self.evidence
    }
}

#[derive(Debug)]
pub struct FlagClaim {
    command_path: CommandPath,
    name: String,
    short: Option<String>,
    summary: Option<String>,
    value_kind: FlagValueKind,
    value_name: Option<String>,
    required: bool,
    repeatable: bool,
    belief: Belief,
    evidence: Vec<String>,
}

#[derive(Debug)]
pub struct OutputContractClaim {
    command_path: CommandPath,
    mode: OutputMode,
    flag_name: String,
    argv_fragment: Vec<String>,
    advertised: bool,
    probed: bool,
    parse_success: bool,
    precondition_blocked: bool,
    preconditions: BTreeSet<PreconditionKind>,
    observed_kind: Option<ObservedOutputKind>,
    diagnostic: Option<String>,
    evidence: Vec<String>,
}

impl OutputContractClaim {
    fn new(
        command_path: CommandPath,
        mode: OutputMode,
        flag_name: String,
        argv_fragment: Vec<String>,
    ) -> Self {
        Self {
            command_path,
            mode,
            flag_name,
            argv_fragment,
            advertised: false,
            probed: false,
            parse_success: false,
            precondition_blocked: false,
            preconditions: BTreeSet::new(),
            observed_kind: None,
            diagnostic: None,
            evidence: Vec::new(),
        }
    }

    fn apply_advertised(&mut self, evidence_id: &str, evidence_detail: &str) {
        self.advertised = true;
        self.evidence
            .push(format!("{evidence_id}:output mode {evidence_detail}"));
    }

    fn apply_probe(&mut self, evidence_id: &str, classification: output::OutputClassification) {
        self.probed = true;
        self.parse_success |= classification.parse_success;
        self.observed_kind = Some(classification.observed_kind);
        self.diagnostic = Some(classification.detail);
        self.evidence
            .push(format!("{evidence_id}:output mode probe"));
    }

    fn apply_precondition_probe(&mut self, evidence_id: &str, precondition: PreconditionKind) {
        self.probed = true;
        self.precondition_blocked = true;
        self.preconditions.insert(precondition);
        self.diagnostic = Some(format!(
            "output-mode probe blocked by {} precondition",
            precondition.label()
        ));
        self.evidence.push(format!(
            "{evidence_id}:output mode blocked by {}",
            precondition.label()
        ));
    }

    pub fn command_path(&self) -> &CommandPath {
        &self.command_path
    }

    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    pub fn flag_name(&self) -> &str {
        &self.flag_name
    }

    pub fn argv_fragment(&self) -> &[String] {
        &self.argv_fragment
    }

    pub fn advertised(&self) -> bool {
        self.advertised
    }

    pub fn probed(&self) -> bool {
        self.probed
    }

    pub fn parse_success(&self) -> bool {
        self.parse_success
    }

    pub fn precondition_blocked(&self) -> bool {
        self.precondition_blocked
    }

    pub fn preconditions(&self) -> impl Iterator<Item = PreconditionKind> + '_ {
        self.preconditions.iter().copied()
    }

    pub fn observed_kind(&self) -> Option<ObservedOutputKind> {
        self.observed_kind
    }

    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    pub fn evidence(&self) -> &[String] {
        &self.evidence
    }
}

impl FlagClaim {
    fn new(command_path: CommandPath, name: String) -> Self {
        Self {
            command_path,
            name,
            short: None,
            summary: None,
            value_kind: FlagValueKind::Boolean,
            value_name: None,
            required: false,
            repeatable: false,
            belief: Belief::with_prior(FLAG_PRIOR),
            evidence: Vec::new(),
        }
    }

    fn apply_layout_candidate(&mut self, candidate: layout::CandidateFlag, evidence_id: &str) {
        self.belief.update(1.0);
        if self.short.is_none() {
            self.short = candidate.short;
        }
        if self.summary.is_none() {
            self.summary = candidate.summary;
        }
        self.value_kind = FlagValueKind::from(candidate.value_kind);
        if self.value_name.is_none() {
            self.value_name = candidate.value_name;
        }
        self.required |= candidate.required;
        self.repeatable |= candidate.repeatable;
        self.evidence
            .push(format!("{evidence_id}:{}", candidate.evidence_detail));
    }

    pub fn command_path(&self) -> &CommandPath {
        &self.command_path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn short(&self) -> Option<&str> {
        self.short.as_deref()
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    pub fn value_kind(&self) -> FlagValueKind {
        self.value_kind
    }

    pub fn value_name(&self) -> Option<&str> {
        self.value_name.as_deref()
    }

    pub fn required(&self) -> bool {
        self.required
    }

    pub fn repeatable(&self) -> bool {
        self.repeatable
    }

    pub fn grammar_known(&self) -> bool {
        self.value_kind == FlagValueKind::Boolean || self.value_name.is_some()
    }

    pub fn confidence(&self) -> f64 {
        self.belief.probability()
    }

    pub fn evidence(&self) -> &[String] {
        &self.evidence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagValueKind {
    Boolean,
    Required,
    Optional,
}

impl From<layout::CandidateFlagValueKind> for FlagValueKind {
    fn from(value: layout::CandidateFlagValueKind) -> Self {
        match value {
            layout::CandidateFlagValueKind::Boolean => Self::Boolean,
            layout::CandidateFlagValueKind::Required => Self::Required,
            layout::CandidateFlagValueKind::Optional => Self::Optional,
        }
    }
}

fn successful_help_like(observation: &ShapeObservation) -> bool {
    matches!(
        &observation.process.status,
        ProcessStatus::Exited { code: Some(0) }
    ) && observation
        .process
        .stdout
        .text
        .as_deref()
        .is_some_and(layout::is_help_like)
}

fn precondition_for_observation(observation: &ShapeObservation) -> Option<PreconditionKind> {
    precondition::classify_process(
        &observation.process.status,
        observation.process.stdout.text.as_deref(),
        observation.process.stderr.text.as_deref(),
    )
}

fn rejected_by_runtime(observation: &ShapeObservation) -> bool {
    matches!(
        &observation.process.status,
        ProcessStatus::Exited { code: Some(code) } if *code != 0
    )
}

fn output_mode_for_intent(intent: ProbeIntent) -> Option<OutputMode> {
    match intent {
        ProbeIntent::OutputJson => Some(OutputMode::Json),
        ProbeIntent::OutputYaml => Some(OutputMode::Yaml),
        ProbeIntent::OutputTable => Some(OutputMode::Table),
        ProbeIntent::OutputPlain => Some(OutputMode::Plain),
        _ => None,
    }
}

fn output_probe_fragment(observation: &ShapeObservation, path: &[String]) -> Vec<String> {
    let mut args = observation
        .process
        .argv
        .iter()
        .skip(1)
        .cloned()
        .collect::<Vec<_>>();
    if args.starts_with(path) {
        args.drain(..path.len());
    }
    if args.last().is_some_and(|arg| arg == "--help") {
        args.pop();
    }
    args
}

fn argv_contains_fragment(argv_fragment: &[String], expected: &[String]) -> bool {
    if expected.is_empty() {
        return true;
    }
    argv_fragment
        .windows(expected.len())
        .any(|window| window == expected)
}

fn absolutize_candidate_path(current_path: &[String], candidate: Vec<String>) -> Vec<String> {
    if current_path.is_empty() || candidate.starts_with(current_path) {
        return candidate;
    }

    let mut full_path = current_path.to_vec();
    full_path.extend(candidate);
    full_path
}

fn is_child_path(current_path: &[String], candidate_path: &[String]) -> bool {
    !current_path.is_empty()
        && candidate_path.len() > current_path.len()
        && candidate_path.starts_with(current_path)
}

#[cfg(test)]
mod tests {
    use super::{ClaimSet, FlagValueKind};
    use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
    use crate::observation::ShapeObservation;
    use crate::precondition::PreconditionKind;
    use crate::process::OutputCapture;

    #[test]
    fn claims_track_layout_and_runtime_confirmation() {
        let observations = vec![
            observation(
                "e_000003",
                ProbeIntent::Help,
                vec![],
                "Commands:\n  measure  Run probes\n",
                Some(0),
            ),
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["measure".to_owned()],
                "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
                Some(0),
            ),
        ];

        let claims = ClaimSet::from_observations("cliare", &observations);
        let measure = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["measure"])
            .expect("measure claim exists");

        assert!(measure.runtime_confirmed());
        assert!(measure.confidence() > 0.90);
        assert!(measure.usage_observed());
        assert!(
            measure
                .positionals()
                .any(|argument| argument.name() == "target" && argument.required())
        );

        let out = claims
            .flags()
            .find(|flag| flag.name() == "--out")
            .expect("out flag exists");
        assert_eq!(out.value_kind(), FlagValueKind::Required);
        assert_eq!(out.value_name(), Some("dir"));
    }

    #[test]
    fn claims_record_negative_diagnostic_probes() {
        let observations = vec![
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["measure".to_owned()],
                "Usage: cliare measure <TARGET>\n\nCommands:\n  nested  Nested command\n",
                Some(0),
            ),
            observation(
                "e_000007",
                ProbeIntent::InvalidChild,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
            observation(
                "e_000009",
                ProbeIntent::InvalidFlag,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
        ];

        let claims = ClaimSet::from_observations("cliare", &observations);
        let measure = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["measure"])
            .expect("measure claim exists");

        assert!(measure.invalid_child_rejected());
        assert!(measure.invalid_flag_rejected());
        assert!(measure.has_child_candidates());
    }

    #[test]
    fn auth_blocked_help_records_precondition_without_negative_help_failure() {
        let observations = vec![
            observation(
                "e_000003",
                ProbeIntent::Help,
                vec![],
                "Commands:\n  model  Track AI model identity\n",
                Some(0),
            ),
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["model".to_owned()],
                "error: rote requires login\n\nrun rote login",
                Some(77),
            ),
        ];

        let claims = ClaimSet::from_observations("rote", &observations);
        let model = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["model"])
            .expect("model claim exists");

        assert!(model.precondition_blocked());
        assert_eq!(
            model.preconditions().collect::<Vec<_>>(),
            vec![PreconditionKind::AuthRequired]
        );
        assert!(!model.help_unavailable());
        assert!(!model.runtime_confirmed());
        assert!(model.confidence() > 0.5);
    }

    fn observation(
        evidence_id: &str,
        intent: ProbeIntent,
        path: Vec<String>,
        stdout: &str,
        exit_code: Option<i32>,
    ) -> ShapeObservation {
        ShapeObservation {
            evidence_id: evidence_id.to_owned(),
            intent,
            path,
            process: ProcessCompleted {
                probe_id: "p_000001".to_owned(),
                argv: vec!["cliare".to_owned(), "--help".to_owned()],
                status: ProcessStatus::Exited { code: exit_code },
                duration_ms: 1,
                stdout: output(stdout),
                stderr: output(""),
                side_effects: crate::sandbox::SideEffectSummary::default(),
            },
        }
    }

    fn output(text: &str) -> OutputCapture {
        OutputCapture {
            sha256: "unused".to_owned(),
            bytes: text.len(),
            retained_bytes: text.len(),
            truncated: false,
            text: Some(text.to_owned()),
        }
    }
}
