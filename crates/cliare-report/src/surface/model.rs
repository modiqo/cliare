use serde::Serialize;

use cliare_cli::cli::SurfaceOutputRequirement;

use super::index::{
    CommandIndexCommand, CommandIndexGap, CommandIndexOutputContract, CommandIndexPositional,
};
use super::matching::{argv_template, cautions, requirements, suggested_flags, use_when};
use super::tokens::TokenSet;

#[derive(Debug, Clone, Serialize)]
pub(super) struct SurfaceMatch {
    pub(super) id: String,
    pub(super) command: String,
    pub(super) path: Vec<String>,
    pub(super) summary: Option<String>,
    pub(super) readiness: String,
    pub(super) runtime_state: String,
    pub(super) confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) match_score: Option<u32>,
    pub(super) argv_template: Vec<String>,
    pub(super) required_positionals: Vec<SurfacePositional>,
    pub(super) suggested_flags: Vec<SurfaceFlag>,
    pub(super) requires: Vec<SurfaceRequirement>,
    pub(super) output_contracts: Vec<SurfaceOutputContract>,
    pub(super) cautions: Vec<String>,
    pub(super) gaps: Vec<SurfaceGap>,
    pub(super) evidence: Vec<String>,
    pub(super) why: String,
    pub(super) use_when: String,
}

impl SurfaceMatch {
    pub(super) fn from_command(
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
pub(super) struct SurfaceRequirement {
    pub(super) kind: &'static str,
    pub(super) name: String,
    pub(super) required: bool,
    pub(super) source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SurfacePositional {
    pub(super) name: String,
    pub(super) required: bool,
    pub(super) variadic: bool,
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
pub(super) struct SurfaceFlag {
    pub(super) name: String,
    pub(super) short: Option<String>,
    pub(super) value_name: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) required: bool,
    pub(super) repeatable: bool,
    pub(super) reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SurfaceOutputContract {
    pub(super) mode: String,
    pub(super) status: String,
    pub(super) argv_fragment: Vec<String>,
    pub(super) observed_kind: Option<String>,
    pub(super) preconditions: Vec<String>,
    pub(super) diagnostic: Option<String>,
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
pub(super) struct SurfaceGap {
    pub(super) kind: String,
    pub(super) reason: String,
}

impl SurfaceGap {
    fn from_gap(gap: &CommandIndexGap) -> Self {
        Self {
            kind: gap.kind.clone(),
            reason: gap.reason.clone(),
        }
    }
}
