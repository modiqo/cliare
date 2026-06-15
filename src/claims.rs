use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::belief::Belief;
use crate::evidence::{ProbeIntent, ProcessStatus};
use crate::layout::{self, CandidateOutputModeScope};
use crate::observation::ShapeObservation;
use crate::output::{self, ObservedOutputKind, OutputMode};
use crate::precondition::{self, PreconditionKind};
use crate::score_model::{ClaimInferenceModel, ScoreModelSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputContractScope {
    GlobalFlag,
    CommandFlag,
    Example,
    RuntimeProbe,
}

impl OutputContractScope {
    fn from_candidate(scope: CandidateOutputModeScope) -> Self {
        match scope {
            CandidateOutputModeScope::CommandFlag => Self::CommandFlag,
            CandidateOutputModeScope::GlobalFlag => Self::GlobalFlag,
            CandidateOutputModeScope::Example => Self::Example,
        }
    }

    fn merge(self, other: Self) -> Self {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::GlobalFlag => 0,
            Self::RuntimeProbe => 1,
            Self::CommandFlag => 2,
            Self::Example => 3,
        }
    }

    pub fn is_global_only(self) -> bool {
        self == Self::GlobalFlag
    }
}

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
    inference: ClaimInferenceModel,
}

impl ClaimSet {
    pub fn from_observations(binary_name: &str, observations: &[ShapeObservation]) -> Self {
        Self::from_observations_with_model(binary_name, observations, ScoreModelSpec::bundled())
    }

    pub fn from_observations_with_model(
        binary_name: &str,
        observations: &[ShapeObservation],
        model: &ScoreModelSpec,
    ) -> Self {
        Self::from_observations_with_inference(
            binary_name,
            observations,
            model.claim_inference_model(),
        )
    }

    fn from_observations_with_inference(
        binary_name: &str,
        observations: &[ShapeObservation],
        inference: ClaimInferenceModel,
    ) -> Self {
        let mut claims = Self {
            commands: BTreeMap::new(),
            flags: BTreeMap::new(),
            outputs: BTreeMap::new(),
            inference,
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
            | ProbeIntent::OutputPlain
            | ProbeIntent::OutputJsonHelp
            | ProbeIntent::OutputYamlHelp
            | ProbeIntent::OutputTableHelp
            | ProbeIntent::OutputPlainHelp => self.apply_output_mode_observation(observation),
            ProbeIntent::Version | ProbeIntent::InvalidCommand => {}
        }
    }

    fn apply_help_observation(&mut self, binary_name: &str, observation: &ShapeObservation) {
        let help_like = successful_help_like(observation);
        let precondition = precondition_for_observation(observation);
        let current_path = CommandPath::new(observation.path.clone());
        let alias_paths = self.alias_paths_for(current_path.as_slice());
        let text = observation.process.stdout.text.as_deref();
        let help_matches_current_path = help_like
            && text.is_some_and(|stdout| {
                layout::help_matches_command_path(stdout, binary_name, current_path.as_slice())
                    || alias_paths.iter().any(|path| {
                        layout::help_matches_command_path(stdout, binary_name, path.as_slice())
                    })
            });

        if !current_path.is_empty() {
            self.command_mut(current_path.clone()).apply_runtime_help(
                observation,
                &observation.evidence_id,
                help_like,
                help_matches_current_path,
                precondition,
            );
        }

        let Some(text) = text else {
            return;
        };
        if !help_like && !current_path.is_empty() {
            return;
        }

        let command_candidates = if current_path.is_empty() || !layout::is_manpage_like(text) {
            layout::command_candidates(text, binary_name)
        } else {
            Vec::new()
        };
        let relative_candidate_scope =
            relative_candidate_scope(current_path.as_slice(), &command_candidates);
        let usage_scope = CommandPath::new(
            layout::usage_command_path(text, binary_name, current_path.as_slice())
                .unwrap_or(relative_candidate_scope.clone()),
        );

        if !usage_scope.is_empty() {
            let arguments = layout::usage_arguments(text, binary_name, usage_scope.as_slice());
            self.command_mut(usage_scope.clone())
                .apply_usage_arguments(arguments, &observation.evidence_id);
        }
        let feature_scope = if usage_scope.is_empty() {
            current_path.clone()
        } else {
            usage_scope.clone()
        };

        if !command_candidates.is_empty() {
            for candidate in command_candidates {
                let path = CommandPath::new(absolutize_candidate_path(
                    relative_candidate_scope.as_slice(),
                    candidate.path,
                    candidate.absolute,
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
            let key = (feature_scope.clone(), candidate.name.clone());
            let inference = self.inference;
            self.flags
                .entry(key)
                .or_insert_with(|| {
                    FlagClaim::new(feature_scope.clone(), candidate.name.clone(), inference)
                })
                .apply_layout_candidate(candidate, &observation.evidence_id);
        }

        for candidate in layout::output_mode_candidates(text) {
            let key = OutputContractKey {
                command_path: feature_scope.clone(),
                mode: candidate.mode,
                argv_fragment: candidate.argv_fragment.clone(),
            };
            self.outputs
                .entry(key)
                .or_insert_with(|| {
                    OutputContractClaim::new(
                        feature_scope.clone(),
                        candidate.mode,
                        candidate.flag_name.clone(),
                        candidate.argv_fragment.clone(),
                        OutputContractScope::from_candidate(candidate.scope),
                    )
                })
                .apply_advertised(
                    &observation.evidence_id,
                    &candidate.evidence_detail,
                    OutputContractScope::from_candidate(candidate.scope),
                );
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
        let help_probe = output_help_probe(observation.intent);
        let path = CommandPath::new(observation.path.clone());
        let precondition = precondition_for_observation(observation);
        if let Some(precondition) = precondition.filter(|_| !help_probe) {
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
                OutputContractClaim::new(
                    path,
                    mode,
                    mode.label().to_owned(),
                    argv_fragment,
                    OutputContractScope::RuntimeProbe,
                )
            });
            if help_probe && let Some(precondition) = precondition {
                claim.apply_help_precondition_probe(&observation.evidence_id, precondition);
            } else if let Some(precondition) = precondition {
                claim.apply_precondition_probe(&observation.evidence_id, precondition);
            } else if help_probe {
                claim.apply_help_probe(
                    &observation.evidence_id,
                    output::classify_help_precedence(
                        mode,
                        &observation.process.status,
                        observation.process.stdout.text.as_deref(),
                    ),
                );
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
                if help_probe && let Some(precondition) = precondition {
                    claim.apply_help_precondition_probe(&observation.evidence_id, precondition);
                } else if let Some(precondition) = precondition {
                    claim.apply_precondition_probe(&observation.evidence_id, precondition);
                } else if help_probe {
                    claim.apply_help_probe(
                        &observation.evidence_id,
                        output::classify_help_precedence(
                            mode,
                            &observation.process.status,
                            observation.process.stdout.text.as_deref(),
                        ),
                    );
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
        let inference = self.inference;
        self.commands
            .entry(path.clone())
            .or_insert_with(|| CommandClaim::new(path, inference))
    }

    fn alias_paths_for(&self, path: &[String]) -> Vec<Vec<String>> {
        let Some(command) = self.commands.get(&CommandPath::new(path.to_vec())) else {
            return Vec::new();
        };
        if path.is_empty() {
            return Vec::new();
        }

        command
            .aliases
            .iter()
            .map(|alias| {
                let mut alias_path = path.to_vec();
                if let Some(last) = alias_path.last_mut() {
                    *last = alias.clone();
                }
                alias_path
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct OutputContractKey {
    command_path: CommandPath,
    mode: OutputMode,
    argv_fragment: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpProbeForm {
    Canonical,
    Alternate,
}

fn help_probe_form(path: &CommandPath, observation: &ShapeObservation) -> HelpProbeForm {
    let argv_tail = observation.process.argv.get(1..).unwrap_or_default();
    if !path.is_empty() && argv_tail.first().is_some_and(|arg| arg == "help") {
        HelpProbeForm::Alternate
    } else {
        HelpProbeForm::Canonical
    }
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
    canonical_help_unavailable: bool,
    alternate_help_unavailable: bool,
    alternate_help_evidence: Vec<String>,
    preconditions: BTreeSet<PreconditionKind>,
    has_child_candidates: bool,
    invalid_child_rejected: bool,
    invalid_flag_rejected: bool,
    evidence: Vec<String>,
    inference: ClaimInferenceModel,
}

impl CommandClaim {
    fn new(path: CommandPath, inference: ClaimInferenceModel) -> Self {
        Self {
            path,
            summary: None,
            aliases: BTreeSet::new(),
            positionals: BTreeMap::new(),
            usage_observed: false,
            belief: Belief::with_prior(inference.command_prior),
            runtime_confirmed: false,
            canonical_help_unavailable: false,
            alternate_help_unavailable: false,
            alternate_help_evidence: Vec::new(),
            preconditions: BTreeSet::new(),
            has_child_candidates: false,
            invalid_child_rejected: false,
            invalid_flag_rejected: false,
            evidence: Vec::new(),
            inference,
        }
    }

    fn apply_layout_candidate(
        &mut self,
        summary: Option<String>,
        aliases: Vec<String>,
        evidence_id: &str,
        evidence_detail: &str,
    ) {
        self.belief.update(self.inference.layout_candidate);
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
        self.belief.update(self.inference.usage_syntax);
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
        observation: &ShapeObservation,
        evidence_id: &str,
        help_like: bool,
        help_matches_current_path: bool,
        precondition: Option<PreconditionKind>,
    ) {
        let probe_form = help_probe_form(&self.path, observation);
        self.apply_runtime_help_with_form(
            evidence_id,
            help_like,
            help_matches_current_path,
            precondition,
            probe_form,
        );
    }

    fn apply_runtime_help_with_form(
        &mut self,
        evidence_id: &str,
        help_like: bool,
        help_matches_current_path: bool,
        precondition: Option<PreconditionKind>,
        probe_form: HelpProbeForm,
    ) {
        if help_matches_current_path {
            self.belief.update(self.inference.runtime_help_match);
            self.runtime_confirmed = true;
            if matches!(probe_form, HelpProbeForm::Canonical) {
                self.canonical_help_unavailable = false;
            }
        } else if let Some(precondition) = precondition {
            self.apply_precondition_blocked(evidence_id, precondition);
            return;
        } else if help_like {
            self.belief.update(self.inference.runtime_help_observed);
            self.evidence.push(format!("{evidence_id}:help observed"));
            return;
        } else if matches!(probe_form, HelpProbeForm::Alternate) {
            self.belief
                .update(self.inference.alternate_help_unavailable);
            self.alternate_help_unavailable = true;
            self.alternate_help_evidence.push(evidence_id.to_owned());
        } else {
            self.belief
                .update(self.inference.non_help_output_from_help_probe);
            self.canonical_help_unavailable = true;
        }
        self.evidence.push(evidence_id.to_owned());
    }

    fn apply_precondition_blocked(&mut self, evidence_id: &str, precondition: PreconditionKind) {
        self.belief
            .update(self.inference.runtime_precondition_block);
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
            self.belief.update(self.inference.runtime_rejection);
        }
        self.evidence.push(evidence_id.to_owned());
    }

    fn apply_invalid_flag(&mut self, evidence_id: &str, rejected: bool) {
        if rejected {
            self.invalid_flag_rejected = true;
            self.belief.update(self.inference.runtime_rejection);
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
        self.canonical_help_unavailable && !self.runtime_confirmed
    }

    pub fn alternate_help_unavailable(&self) -> bool {
        self.alternate_help_unavailable && self.runtime_confirmed
    }

    pub fn alternate_help_evidence(&self) -> &[String] {
        &self.alternate_help_evidence
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
    inference: ClaimInferenceModel,
}

#[derive(Debug)]
pub struct OutputContractClaim {
    command_path: CommandPath,
    mode: OutputMode,
    flag_name: String,
    argv_fragment: Vec<String>,
    scope: OutputContractScope,
    advertised: bool,
    probed: bool,
    parse_success: bool,
    precondition_blocked: bool,
    preconditions: BTreeSet<PreconditionKind>,
    observed_kind: Option<ObservedOutputKind>,
    diagnostic: Option<String>,
    help_probed: bool,
    help_behavior: Option<output::OutputHelpBehavior>,
    help_parse_success: bool,
    help_diagnostic: Option<String>,
    evidence: Vec<String>,
}

impl OutputContractClaim {
    fn new(
        command_path: CommandPath,
        mode: OutputMode,
        flag_name: String,
        argv_fragment: Vec<String>,
        scope: OutputContractScope,
    ) -> Self {
        Self {
            command_path,
            mode,
            flag_name,
            argv_fragment,
            scope,
            advertised: false,
            probed: false,
            parse_success: false,
            precondition_blocked: false,
            preconditions: BTreeSet::new(),
            observed_kind: None,
            diagnostic: None,
            help_probed: false,
            help_behavior: None,
            help_parse_success: false,
            help_diagnostic: None,
            evidence: Vec::new(),
        }
    }

    fn apply_advertised(
        &mut self,
        evidence_id: &str,
        evidence_detail: &str,
        scope: OutputContractScope,
    ) {
        self.advertised = true;
        self.scope = self.scope.merge(scope);
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

    fn apply_help_probe(
        &mut self,
        evidence_id: &str,
        classification: output::OutputHelpClassification,
    ) {
        self.help_probed = true;
        self.help_parse_success |= classification.parse_success;
        self.help_behavior = Some(classification.behavior);
        self.help_diagnostic = Some(classification.detail);
        self.evidence
            .push(format!("{evidence_id}:output mode help probe"));
    }

    fn apply_help_precondition_probe(&mut self, evidence_id: &str, precondition: PreconditionKind) {
        self.help_probed = true;
        self.help_behavior = Some(output::OutputHelpBehavior::PreconditionBlocked);
        self.help_diagnostic = Some(format!(
            "output-help probe blocked by {} precondition",
            precondition.label()
        ));
        self.evidence.push(format!(
            "{evidence_id}:output mode help blocked by {}",
            precondition.label()
        ));
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

    pub fn scope(&self) -> OutputContractScope {
        self.scope
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

    pub fn help_probed(&self) -> bool {
        self.help_probed
    }

    pub fn help_behavior(&self) -> Option<output::OutputHelpBehavior> {
        self.help_behavior
    }

    pub fn help_parse_success(&self) -> bool {
        self.help_parse_success
    }

    pub fn help_diagnostic(&self) -> Option<&str> {
        self.help_diagnostic.as_deref()
    }

    pub fn evidence(&self) -> &[String] {
        &self.evidence
    }
}

impl FlagClaim {
    fn new(command_path: CommandPath, name: String, inference: ClaimInferenceModel) -> Self {
        Self {
            command_path,
            name,
            short: None,
            summary: None,
            value_kind: FlagValueKind::Boolean,
            value_name: None,
            required: false,
            repeatable: false,
            belief: Belief::with_prior(inference.flag_prior),
            evidence: Vec::new(),
            inference,
        }
    }

    fn apply_layout_candidate(&mut self, candidate: layout::CandidateFlag, evidence_id: &str) {
        self.belief.update(self.inference.layout_candidate);
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
        ProbeIntent::OutputJson | ProbeIntent::OutputJsonHelp => Some(OutputMode::Json),
        ProbeIntent::OutputYaml | ProbeIntent::OutputYamlHelp => Some(OutputMode::Yaml),
        ProbeIntent::OutputTable | ProbeIntent::OutputTableHelp => Some(OutputMode::Table),
        ProbeIntent::OutputPlain | ProbeIntent::OutputPlainHelp => Some(OutputMode::Plain),
        _ => None,
    }
}

fn output_help_probe(intent: ProbeIntent) -> bool {
    matches!(
        intent,
        ProbeIntent::OutputJsonHelp
            | ProbeIntent::OutputYamlHelp
            | ProbeIntent::OutputTableHelp
            | ProbeIntent::OutputPlainHelp
    )
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

fn relative_candidate_scope(
    current_path: &[String],
    candidates: &[layout::CandidateCommand],
) -> Vec<String> {
    if current_path.is_empty() {
        return Vec::new();
    }

    let relative_candidates = candidates
        .iter()
        .filter(|candidate| !candidate.absolute)
        .collect::<Vec<_>>();
    if relative_candidates.is_empty() {
        return current_path.to_vec();
    }

    for prefix_len in (0..current_path.len()).rev() {
        let suffix = &current_path[prefix_len..];
        if relative_candidates
            .iter()
            .any(|candidate| candidate.path.as_slice() == suffix)
        {
            return current_path[..prefix_len].to_vec();
        }
    }

    current_path.to_vec()
}

fn absolutize_candidate_path(
    current_path: &[String],
    candidate: Vec<String>,
    absolute: bool,
) -> Vec<String> {
    if absolute || current_path.is_empty() || candidate.starts_with(current_path) {
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
    use crate::score_model::ScoreModelSpec;

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
    fn multiline_usage_help_confirms_current_command() {
        let observations = vec![observation(
            "e_000024",
            ProbeIntent::Help,
            vec!["backups".to_owned()],
            "Manage Supabase physical backups\n\nUsage:\n  supabase backups [command]\n\nAvailable Commands:\n  list     Lists available physical backups\n  restore  Restore to a specific timestamp using PITR\n\nFlags:\n  -h, --help  help for backups\n",
            Some(0),
        )];

        let claims = ClaimSet::from_observations("supabase", &observations);
        let backups = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["backups"])
            .expect("backups claim exists");

        assert!(backups.runtime_confirmed());
        assert!(!backups.help_unavailable());
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
    fn absolute_help_references_are_not_nested_under_current_command() {
        let observations = vec![observation(
            "e_000011",
            ProbeIntent::Help,
            vec!["adapter".to_owned(), "list".to_owned()],
            "rote adapter list - Show installed adapters\n\nNEXT STEPS\n  rote adapter info <ID>    Show details\n  rote adapter reauth <ID>  Re-authorize an adapter\n",
            Some(0),
        )];

        let claims = ClaimSet::from_observations("rote", &observations);

        assert!(
            claims
                .commands()
                .any(|claim| claim.path().as_slice() == ["adapter", "info"])
        );
        assert!(
            !claims
                .commands()
                .any(|claim| claim.path().as_slice() == ["adapter", "list", "adapter", "info"])
        );
        let adapter_list = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["adapter", "list"])
            .expect("current command claim exists");
        assert!(!adapter_list.has_child_candidates());
    }

    #[test]
    fn parent_help_tables_are_scoped_to_the_matching_ancestor() {
        let observations = vec![observation(
            "e_000013",
            ProbeIntent::Help,
            vec!["flow".to_owned(), "search".to_owned()],
            "FLOW COMMANDS\n\nCOMMANDS:\n  list [--json]        List flows\n  search <QUERY>       Search flows\n  doctor               Check all flows\n",
            Some(0),
        )];

        let claims = ClaimSet::from_observations("rote", &observations);

        assert!(
            claims
                .commands()
                .any(|claim| claim.path().as_slice() == ["flow", "doctor"])
        );
        assert!(
            !claims
                .commands()
                .any(|claim| claim.path().as_slice() == ["flow", "search", "doctor"])
        );
        let flow_search = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["flow", "search"])
            .expect("current command claim exists");
        assert!(!flow_search.has_child_candidates());
    }

    #[test]
    fn parent_help_echo_does_not_confirm_or_attach_features_to_child_candidate() {
        let observations = vec![observation(
            "e_000017",
            ProbeIntent::Help,
            vec![
                "adapter".to_owned(),
                "set".to_owned(),
                "base_url".to_owned(),
            ],
            "tool adapter set - Mutate a key\n\nUSAGE\n  tool adapter set <ID> <KEY> <VALUE> [--json]\n\nFLAGS\n  --json  Emit result as JSON\n",
            Some(0),
        )];

        let claims = ClaimSet::from_observations("tool", &observations);
        let child = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["adapter", "set", "base_url"])
            .expect("child claim exists");

        assert!(!child.runtime_confirmed());
        assert!(!child.help_unavailable());
        assert!(claims.flags().any(
            |claim| claim.command_path().as_slice() == ["adapter", "set"]
                && claim.name() == "--json"
        ));
        assert!(!claims.flags().any(|claim| claim.command_path().as_slice()
            == ["adapter", "set", "base_url"]
            && claim.name() == "--json"));
    }

    #[test]
    fn single_parent_help_row_does_not_duplicate_current_tail() {
        let observations = vec![observation(
            "e_000015",
            ProbeIntent::Help,
            vec!["install".to_owned(), "skill".to_owned()],
            "rote install - Installation utilities\n\nSUBCOMMANDS\n  skill  Install provider integration\n",
            Some(0),
        )];

        let claims = ClaimSet::from_observations("rote", &observations);

        assert!(
            claims
                .commands()
                .any(|claim| claim.path().as_slice() == ["install", "skill"])
        );
        assert!(
            !claims
                .commands()
                .any(|claim| claim.path().as_slice() == ["install", "skill", "skill"])
        );
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

    #[test]
    fn claim_confidence_uses_supplied_model_inference_weights() {
        let observations = vec![observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["model".to_owned()],
            "error: rote requires login\n\nrun rote login",
            Some(77),
        )];
        let default_claims = ClaimSet::from_observations("rote", &observations);
        let default_model = default_claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["model"])
            .expect("default model claim exists");
        assert!(default_model.confidence() > 0.08);

        let mut score_model = ScoreModelSpec::bundled().clone();
        score_model.evidence_weights.runtime_precondition_block = 0.0;

        let custom_claims =
            ClaimSet::from_observations_with_model("rote", &observations, &score_model);
        let custom_model = custom_claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["model"])
            .expect("custom model claim exists");

        assert!(custom_model.precondition_blocked());
        assert!((custom_model.confidence() - 0.08).abs() < 0.000_000_000_001);
    }

    #[test]
    fn flag_confidence_uses_supplied_model_inference_weights() {
        let observations = vec![observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Usage: cliare metadata [--format <FORMAT>]\n\nOptions:\n  --format <FORMAT>  Output format\n",
            Some(0),
        )];
        let mut score_model = ScoreModelSpec::bundled().clone();
        score_model.claim_priors.flag_exists = 0.5;
        score_model.evidence_weights.layout_candidate = 0.0;

        let claims = ClaimSet::from_observations_with_model("cliare", &observations, &score_model);
        let format = claims
            .flags()
            .find(|flag| flag.name() == "--format")
            .expect("format flag exists");

        assert!((format.confidence() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn output_mode_probe_records_runtime_precondition() {
        let path = vec!["issue".to_owned(), "list".to_owned()];
        let observations = vec![
            observation_with_argv_streams(
                "e_000011",
                ProbeIntent::Help,
                path.clone(),
                vec![
                    "gh".to_owned(),
                    "issue".to_owned(),
                    "list".to_owned(),
                    "--help".to_owned(),
                ],
                "USAGE\n  gh issue list [flags]\n\nFLAGS\n      --json fields        Output JSON with the specified fields\n",
                "",
                Some(0),
            ),
            observation_with_argv_streams(
                "e_000013",
                ProbeIntent::OutputJson,
                path.clone(),
                vec![
                    "gh".to_owned(),
                    "issue".to_owned(),
                    "list".to_owned(),
                    "--json".to_owned(),
                    "json".to_owned(),
                ],
                "",
                "gh: To use GitHub CLI in automation, set the GH_TOKEN environment variable.",
                Some(4),
            ),
        ];

        let claims = ClaimSet::from_observations("gh", &observations);
        let issue_list = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["issue", "list"])
            .expect("command claim exists");
        assert!(issue_list.precondition_blocked());
        assert_eq!(
            issue_list.preconditions().collect::<Vec<_>>(),
            vec![PreconditionKind::AuthRequired]
        );

        let contract = claims
            .output_contracts()
            .find(|claim| {
                claim.command_path().as_slice() == ["issue", "list"]
                    && claim.flag_name() == "--json"
            })
            .expect("output contract exists");
        assert!(contract.probed());
        assert!(contract.precondition_blocked());
        assert!(!contract.parse_success());
        assert_eq!(
            contract.preconditions().collect::<Vec<_>>(),
            vec![PreconditionKind::AuthRequired]
        );
    }

    #[test]
    fn output_mode_probe_records_network_precondition() {
        let path = vec!["issue".to_owned(), "list".to_owned()];
        let observations = vec![
            observation_with_argv_streams(
                "e_000011",
                ProbeIntent::Help,
                path.clone(),
                vec![
                    "gh".to_owned(),
                    "issue".to_owned(),
                    "list".to_owned(),
                    "--help".to_owned(),
                ],
                "USAGE\n  gh issue list [flags]\n\nFLAGS\n      --json fields        Output JSON with the specified fields\n",
                "",
                Some(0),
            ),
            observation_with_argv_streams(
                "e_000013",
                ProbeIntent::OutputJson,
                path,
                vec![
                    "gh".to_owned(),
                    "issue".to_owned(),
                    "list".to_owned(),
                    "--json".to_owned(),
                    "number,title,url".to_owned(),
                ],
                "",
                "error connecting to api.example.com\ncheck your internet connection",
                Some(1),
            ),
        ];

        let claims = ClaimSet::from_observations("gh", &observations);
        let contract = claims
            .output_contracts()
            .find(|claim| {
                claim.command_path().as_slice() == ["issue", "list"]
                    && claim
                        .preconditions()
                        .any(|kind| kind == PreconditionKind::NetworkUnavailable)
            })
            .expect("network-blocked output contract exists");

        assert!(contract.probed());
        assert!(contract.precondition_blocked());
        assert!(!contract.parse_success());
        assert_eq!(
            contract.preconditions().collect::<Vec<_>>(),
            vec![PreconditionKind::NetworkUnavailable]
        );
    }

    fn observation(
        evidence_id: &str,
        intent: ProbeIntent,
        path: Vec<String>,
        stdout: &str,
        exit_code: Option<i32>,
    ) -> ShapeObservation {
        observation_with_argv_streams(
            evidence_id,
            intent,
            path,
            vec!["cliare".to_owned(), "--help".to_owned()],
            stdout,
            "",
            exit_code,
        )
    }

    fn observation_with_argv_streams(
        evidence_id: &str,
        intent: ProbeIntent,
        path: Vec<String>,
        argv: Vec<String>,
        stdout: &str,
        stderr: &str,
        exit_code: Option<i32>,
    ) -> ShapeObservation {
        ShapeObservation {
            evidence_id: evidence_id.to_owned(),
            intent,
            path,
            process: ProcessCompleted {
                probe_id: "p_000001".to_owned(),
                argv,
                status: ProcessStatus::Exited { code: exit_code },
                duration_ms: 1,
                stdout: output(stdout),
                stderr: output(stderr),
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
