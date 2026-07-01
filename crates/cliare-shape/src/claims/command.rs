use std::collections::{BTreeMap, BTreeSet};

use cliare_inference::belief::Belief;
use cliare_inference::layout;
use cliare_inference::precondition::PreconditionKind;
use cliare_inference::score_model::ClaimInferenceModel;

use crate::observation::ShapeObservation;

use super::path::CommandPath;

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
    pub(super) path: CommandPath,
    pub(super) summary: Option<String>,
    pub(super) aliases: BTreeSet<String>,
    pub(super) positionals: BTreeMap<String, PositionalClaim>,
    pub(super) usage_observed: bool,
    pub(super) belief: Belief,
    pub(super) runtime_confirmed: bool,
    pub(super) canonical_help_unavailable: bool,
    pub(super) alternate_help_unavailable: bool,
    pub(super) alternate_help_evidence: Vec<String>,
    pub(super) preconditions: BTreeSet<PreconditionKind>,
    pub(super) has_child_candidates: bool,
    pub(super) invalid_child_rejected: bool,
    pub(super) invalid_flag_rejected: bool,
    pub(super) evidence: Vec<String>,
    pub(super) inference: ClaimInferenceModel,
}

impl CommandClaim {
    pub(super) fn new(path: CommandPath, inference: ClaimInferenceModel) -> Self {
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

    pub(super) fn apply_layout_candidate(
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

    pub(super) fn apply_usage_arguments(
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

    pub(super) fn apply_runtime_help(
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

    pub(super) fn apply_precondition_blocked(
        &mut self,
        evidence_id: &str,
        precondition: PreconditionKind,
    ) {
        self.belief
            .update(self.inference.runtime_precondition_block);
        self.preconditions.insert(precondition);
        self.evidence
            .push(format!("{evidence_id}:{}", precondition.label()));
    }

    pub(super) fn apply_child_candidate(&mut self, evidence_id: &str) {
        self.has_child_candidates = true;
        self.evidence
            .push(format!("{evidence_id}:child command row"));
    }

    pub(super) fn apply_invalid_child(&mut self, evidence_id: &str, rejected: bool) {
        if rejected {
            self.invalid_child_rejected = true;
            self.belief.update(self.inference.runtime_rejection);
        }
        self.evidence.push(evidence_id.to_owned());
    }

    pub(super) fn apply_invalid_flag(&mut self, evidence_id: &str, rejected: bool) {
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
    pub(super) evidence: Vec<String>,
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
