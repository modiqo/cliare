use cliare_inference::belief::Belief;
use cliare_inference::layout;
use cliare_inference::score_model::ClaimInferenceModel;

use super::path::CommandPath;

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

impl FlagClaim {
    pub(super) fn new(
        command_path: CommandPath,
        name: String,
        inference: ClaimInferenceModel,
    ) -> Self {
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

    pub(super) fn apply_layout_candidate(
        &mut self,
        candidate: layout::CandidateFlag,
        evidence_id: &str,
    ) {
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
