use std::collections::BTreeMap;

use cliare_core::probe_intent::ProbeIntent;
use cliare_inference::layout;
use cliare_inference::output;
use cliare_inference::score_model::{ClaimInferenceModel, ScoreModelSpec};

use crate::observation::ShapeObservation;

use super::command::CommandClaim;
use super::flag::FlagClaim;
use super::helpers::{
    absolutize_candidate_path, argv_contains_fragment, is_child_path, output_help_probe,
    output_mode_for_intent, output_probe_fragment, precondition_for_observation,
    rejected_by_runtime, relative_candidate_scope, successful_help_like,
};
use super::output_contract::{OutputContractClaim, OutputContractKey, OutputContractScope};
use super::path::CommandPath;

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
