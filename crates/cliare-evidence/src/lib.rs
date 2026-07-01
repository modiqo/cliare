use serde::{Deserialize, Serialize};

pub use cliare_core::probe_intent::ProbeIntent;
pub use cliare_core::process_status::ProcessStatus;
use cliare_runtime::process::{OutputCapture, ProbeOutcome};
use cliare_runtime::sandbox::SideEffectSummary;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessCompleted {
    pub probe_id: String,
    pub argv: Vec<String>,
    pub status: ProcessStatus,
    pub duration_ms: u128,
    pub stdout: OutputCapture,
    pub stderr: OutputCapture,
    pub side_effects: SideEffectSummary,
}

impl ProcessCompleted {
    pub fn from_outcome(probe_id: String, outcome: ProbeOutcome) -> Self {
        let status = if outcome.timed_out {
            ProcessStatus::TimedOut
        } else {
            ProcessStatus::Exited {
                code: outcome.exit_code,
            }
        };

        Self {
            probe_id,
            argv: outcome.argv,
            status,
            duration_ms: outcome.duration.as_millis(),
            stdout: outcome.stdout,
            stderr: outcome.stderr,
            side_effects: outcome.side_effects,
        }
    }
}
