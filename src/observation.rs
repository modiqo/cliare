use crate::evidence::{ProbeIntent, ProcessCompleted};

#[derive(Debug, Clone)]
pub struct ShapeObservation {
    pub evidence_id: String,
    pub intent: ProbeIntent,
    pub path: Vec<String>,
    pub process: ProcessCompleted,
}
