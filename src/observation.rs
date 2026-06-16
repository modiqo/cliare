use crate::evidence::{ProbeIntent, ProcessCompleted};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShapeObservation {
    pub evidence_id: String,
    pub intent: ProbeIntent,
    pub path: Vec<String>,
    pub process: ProcessCompleted,
}
