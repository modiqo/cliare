use std::collections::BTreeSet;

use serde::Serialize;

use cliare_inference::layout::CandidateOutputModeScope;
use cliare_inference::output::{self, ObservedOutputKind, OutputMode};
use cliare_inference::precondition::PreconditionKind;

use super::path::CommandPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputContractScope {
    GlobalFlag,
    CommandFlag,
    Example,
    RuntimeProbe,
}

impl OutputContractScope {
    pub(super) fn from_candidate(scope: CandidateOutputModeScope) -> Self {
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
pub(super) struct OutputContractKey {
    pub(super) command_path: CommandPath,
    pub(super) mode: OutputMode,
    pub(super) argv_fragment: Vec<String>,
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
    pub(super) fn new(
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

    pub(super) fn apply_advertised(
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

    pub(super) fn apply_probe(
        &mut self,
        evidence_id: &str,
        classification: output::OutputClassification,
    ) {
        self.probed = true;
        self.parse_success |= classification.parse_success;
        self.observed_kind = Some(classification.observed_kind);
        self.diagnostic = Some(classification.detail);
        self.evidence
            .push(format!("{evidence_id}:output mode probe"));
    }

    pub(super) fn apply_help_probe(
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

    pub(super) fn apply_help_precondition_probe(
        &mut self,
        evidence_id: &str,
        precondition: PreconditionKind,
    ) {
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

    pub(super) fn apply_precondition_probe(
        &mut self,
        evidence_id: &str,
        precondition: PreconditionKind,
    ) {
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
