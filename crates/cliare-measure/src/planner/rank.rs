use std::cmp::Ordering;

use crate::claims::{CommandClaim, OutputContractClaim};
use crate::evidence::ProbeIntent;
use crate::output::OutputMode;
use crate::process::ProbeSpec;

#[derive(Debug, Clone)]
pub(super) struct ProbePlan {
    pub(super) rank: PlannerRank,
    pub(super) probe: ProbeSpec,
    pub(super) expected_value: u16,
}

impl ProbePlan {
    pub(super) fn new(probe: ProbeSpec, rank: PlannerRank) -> Self {
        Self {
            rank,
            probe,
            expected_value: rank.expected_value,
        }
    }

    pub(super) fn bootstrap(probe: ProbeSpec) -> Self {
        Self {
            rank: PlannerRank::bootstrap(),
            probe,
            expected_value: 1_000,
        }
    }
}

impl Ord for ProbePlan {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank
            .cmp(&other.rank)
            .then_with(|| other.expected_value.cmp(&self.expected_value))
            .then_with(|| self.probe.args.cmp(&other.probe.args))
    }
}

impl PartialOrd for ProbePlan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ProbePlan {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank
            && self.expected_value == other.expected_value
            && self.probe.args == other.probe.args
    }
}

impl Eq for ProbePlan {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PlannerRank {
    category: u8,
    expected_value: u16,
    uncertainty: u16,
    confidence: u16,
    depth: u16,
    intent_order: u8,
}

impl PlannerRank {
    fn bootstrap() -> Self {
        Self {
            category: 0,
            expected_value: 1_000,
            uncertainty: 1_000,
            confidence: 1_000,
            depth: 0,
            intent_order: 0,
        }
    }

    pub(super) fn for_help_confirmation(claim: &CommandClaim, intent_order: u8) -> Self {
        let confidence = quantized_confidence(claim.confidence());
        let uncertainty = uncertainty(confidence);
        Self {
            category: 0,
            expected_value: help_expected_value(confidence, uncertainty),
            uncertainty,
            confidence,
            depth: claim.path().len() as u16,
            intent_order,
        }
    }

    pub(super) fn for_diagnostic_probe(claim: &CommandClaim, intent_order: u8) -> Self {
        Self {
            category: 1,
            expected_value: diagnostic_expected_value(quantized_confidence(claim.confidence())),
            uncertainty: 0,
            confidence: quantized_confidence(claim.confidence()),
            depth: claim.path().len() as u16,
            intent_order,
        }
    }

    pub(super) fn for_output_probe(claim: &OutputContractClaim, intent_order: u8) -> Self {
        Self {
            category: 2,
            expected_value: output_expected_value(claim.mode()),
            uncertainty: if claim.probed() { 0 } else { 700 },
            confidence: if claim.advertised() { 800 } else { 200 },
            depth: claim.command_path().len() as u16,
            intent_order: output_intent_order(claim.mode(), intent_order),
        }
    }
}

impl Ord for PlannerRank {
    fn cmp(&self, other: &Self) -> Ordering {
        self.category
            .cmp(&other.category)
            .then_with(|| other.expected_value.cmp(&self.expected_value))
            .then_with(|| other.uncertainty.cmp(&self.uncertainty))
            .then_with(|| other.confidence.cmp(&self.confidence))
            .then_with(|| self.depth.cmp(&other.depth))
            .then_with(|| self.intent_order.cmp(&other.intent_order))
    }
}

impl PartialOrd for PlannerRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn quantized_confidence(confidence: f64) -> u16 {
    (confidence.clamp(0.0, 1.0) * 1_000.0).round() as u16
}

fn uncertainty(confidence: u16) -> u16 {
    500_u16.saturating_sub(500_u16.abs_diff(confidence))
}

fn help_expected_value(confidence: u16, uncertainty: u16) -> u16 {
    uncertainty.saturating_add(confidence / 4).min(1_000)
}

fn diagnostic_expected_value(confidence: u16) -> u16 {
    200_u16.saturating_add(confidence / 2).min(1_000)
}

fn output_expected_value(mode: OutputMode) -> u16 {
    match mode {
        OutputMode::Json => 700,
        OutputMode::Yaml => 550,
        OutputMode::Table => 250,
        OutputMode::Plain => 150,
    }
}

pub(super) fn output_intent(mode: OutputMode) -> Option<ProbeIntent> {
    match mode {
        OutputMode::Json => Some(ProbeIntent::OutputJson),
        OutputMode::Yaml => Some(ProbeIntent::OutputYaml),
        OutputMode::Table => Some(ProbeIntent::OutputTable),
        OutputMode::Plain => Some(ProbeIntent::OutputPlain),
    }
}

pub(super) fn output_help_intent(mode: OutputMode) -> Option<ProbeIntent> {
    match mode {
        OutputMode::Json => Some(ProbeIntent::OutputJsonHelp),
        OutputMode::Yaml => Some(ProbeIntent::OutputYamlHelp),
        OutputMode::Table => Some(ProbeIntent::OutputTableHelp),
        OutputMode::Plain => Some(ProbeIntent::OutputPlainHelp),
    }
}

fn output_intent_order(mode: OutputMode, probe_order: u8) -> u8 {
    let mode_order = match mode {
        OutputMode::Json => 0,
        OutputMode::Yaml => 1,
        OutputMode::Table => 2,
        OutputMode::Plain => 3,
    };
    mode_order * 2 + probe_order
}
