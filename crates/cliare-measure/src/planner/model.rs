use crate::claims::ClaimSet;
use crate::process::ProbeSpec;

pub trait ProbePlanner {
    fn seed(&mut self, probes: impl IntoIterator<Item = ProbeSpec>);
    fn extend_from_claims(&mut self, claims: &ClaimSet);
    fn next(&mut self) -> Option<ProbeSpec>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvergencePolicy {
    pub min_expected_value: u16,
}

impl ConvergencePolicy {
    pub fn new(min_expected_value: u16) -> Self {
        Self { min_expected_value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannerStats {
    pub max_depth: usize,
    pub min_expected_value: u16,
    pub frontier_remaining: usize,
    pub highest_pending_expected_value: Option<u16>,
    pub candidates_skipped_by_depth: usize,
    pub candidates_skipped_by_convergence: usize,
}
