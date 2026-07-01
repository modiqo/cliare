use serde::{Deserialize, Serialize};

use crate::cli::{MeasureArgs, TraversalProfile};
use crate::context::RuntimeContext;
use crate::sandbox::SnapshotLimits;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(super) struct ProbeProfile {
    pub(super) traversal_profile: TraversalProfile,
    pub(super) sandbox_profile: String,
    #[serde(default)]
    pub(super) runtime_context: RuntimeContext,
    pub(super) timeout_ms: u64,
    pub(super) output_limit_bytes: usize,
    pub(super) max_depth: usize,
    pub(super) max_probes: usize,
    pub(super) min_expected_value: u16,
    #[serde(default)]
    pub(super) concurrency_limit: usize,
    #[serde(default)]
    pub(super) snapshot_limits: SnapshotLimits,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResolvedProbeProfile {
    pub(super) max_depth: usize,
    pub(super) max_probes: usize,
    pub(super) min_expected_value: u16,
    pub(super) concurrency_limit: usize,
    pub(super) snapshot_limits: SnapshotLimits,
}

impl ProbeProfile {
    pub(super) fn from_args(
        args: &MeasureArgs,
        resolved: ResolvedProbeProfile,
        sandbox_profile: &str,
        runtime_context: RuntimeContext,
    ) -> Self {
        Self {
            traversal_profile: args.profile,
            sandbox_profile: sandbox_profile.to_owned(),
            runtime_context,
            timeout_ms: args.timeout_ms,
            output_limit_bytes: args.output_limit_bytes,
            max_depth: resolved.max_depth,
            max_probes: resolved.max_probes,
            min_expected_value: resolved.min_expected_value,
            concurrency_limit: resolved.concurrency_limit,
            snapshot_limits: resolved.snapshot_limits,
        }
    }
}
