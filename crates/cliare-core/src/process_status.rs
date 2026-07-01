use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProcessStatus {
    Exited { code: Option<i32> },
    TimedOut,
    SpawnFailed { error: String },
}
