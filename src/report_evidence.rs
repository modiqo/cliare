use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use tokio::fs;

use crate::error::{CliareError, Result};

const EVIDENCE_SAMPLE_LIMIT: usize = 50;

#[derive(Debug, Default)]
pub(crate) struct EvidenceSummary {
    pub(crate) probes_scheduled: usize,
    pub(crate) processes_completed: usize,
    pub(crate) scheduled: BTreeMap<String, ScheduledProbe>,
    pub(crate) processes: BTreeMap<String, ProcessEvidence>,
    pub(crate) probe_failures: Vec<ProbeFailure>,
    pub(crate) side_effects: Vec<SideEffectRecord>,
}

impl EvidenceSummary {
    pub(crate) async fn read(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path)
                .await
                .map_err(|source| CliareError::ReadReportArtifact {
                    path: path.to_path_buf(),
                    source,
                })?;
        let mut summary = Self::default();

        for (line_index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let value: Value =
                serde_json::from_str(line).map_err(|source| CliareError::ParseReportEvidence {
                    path: path.to_path_buf(),
                    line: line_index + 1,
                    source,
                })?;
            match value["kind"].as_str() {
                Some("probe_scheduled") => summary.record_scheduled(&value),
                Some("process_completed") => summary.record_process(&value),
                _ => {}
            }
        }

        Ok(summary)
    }

    fn record_scheduled(&mut self, value: &Value) {
        let payload = &value["payload"];
        let Some(probe_id) = payload["probe_id"].as_str() else {
            return;
        };
        self.probes_scheduled += 1;
        self.scheduled.insert(
            probe_id.to_owned(),
            ScheduledProbe {
                intent: payload["intent"].as_str().map(str::to_owned),
                path: string_array(&payload["path"]),
                argv: string_array(&payload["argv"]),
            },
        );
    }

    fn record_process(&mut self, value: &Value) {
        let payload = &value["payload"];
        let Some(probe_id) = payload["probe_id"].as_str() else {
            return;
        };
        self.processes_completed += 1;

        let scheduled = self.scheduled.get(probe_id);
        let argv = string_array(&payload["argv"]);
        let command_path = scheduled.map_or_else(Vec::new, |probe| probe.path.clone());
        let intent = scheduled.and_then(|probe| probe.intent.clone());
        let status = status_label(&payload["status"]);
        let status_state = payload["status"]["state"].as_str();
        let evidence_id = value["event_id"].as_str().unwrap_or_default().to_owned();
        self.processes.insert(
            evidence_id.clone(),
            ProcessEvidence {
                probe_id: probe_id.to_owned(),
                intent: intent.clone(),
                path: command_path.clone(),
                argv: if argv.is_empty() {
                    scheduled.map_or_else(Vec::new, |probe| probe.argv.clone())
                } else {
                    argv.clone()
                },
                status: status.clone(),
            },
        );
        if matches!(status_state, Some("timed_out" | "spawn_failed")) {
            self.probe_failures.push(ProbeFailure {
                probe_id: probe_id.to_owned(),
                intent: intent.clone(),
                path: command_path.clone(),
                argv: if argv.is_empty() {
                    scheduled.map_or_else(Vec::new, |probe| probe.argv.clone())
                } else {
                    argv.clone()
                },
                status: status.clone(),
                evidence: evidence_id.clone(),
            });
        }

        let Some(changes) = payload["side_effects"]["changes"].as_array() else {
            return;
        };
        for change in changes {
            let Some(change_path) = change["path"].as_str() else {
                continue;
            };
            self.side_effects.push(SideEffectRecord {
                evidence: evidence_id.clone(),
                probe_id: probe_id.to_owned(),
                intent: intent.clone(),
                command_path: command_path.clone(),
                argv: if argv.is_empty() {
                    scheduled.map_or_else(Vec::new, |probe| probe.argv.clone())
                } else {
                    argv.clone()
                },
                kind: change["kind"].as_str().unwrap_or("unknown").to_owned(),
                region: change["region"].as_str().unwrap_or("unknown").to_owned(),
                path: change_path.to_owned(),
                size_bytes: change["size_bytes"].as_u64(),
            });
        }
    }
}

#[derive(Debug)]
pub(crate) struct ScheduledProbe {
    pub(crate) intent: Option<String>,
    pub(crate) path: Vec<String>,
    pub(crate) argv: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProcessEvidence {
    pub(crate) probe_id: String,
    pub(crate) intent: Option<String>,
    pub(crate) path: Vec<String>,
    pub(crate) argv: Vec<String>,
    pub(crate) status: String,
}

impl ProcessEvidence {
    pub(crate) fn summary(&self) -> String {
        format!(
            "probe `{}` for {} completed with `{}`",
            self.probe_id,
            self.scope_label(),
            self.status
        )
    }

    pub(crate) fn scope_label(&self) -> String {
        let path = if self.path.is_empty() {
            return "root command".to_owned();
        } else {
            self.path.join(" ")
        };
        format!("command `{path}`")
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct EvidenceSummaryPacket {
    pub(crate) probes_scheduled: usize,
    pub(crate) processes_completed: usize,
    pub(crate) probe_failures_total: usize,
    pub(crate) probe_failure_samples: Vec<ProbeFailure>,
    pub(crate) side_effects_total: usize,
    pub(crate) side_effect_samples: Vec<SideEffectRecord>,
}

impl From<&EvidenceSummary> for EvidenceSummaryPacket {
    fn from(summary: &EvidenceSummary) -> Self {
        Self {
            probes_scheduled: summary.probes_scheduled,
            processes_completed: summary.processes_completed,
            probe_failures_total: summary.probe_failures.len(),
            probe_failure_samples: summary
                .probe_failures
                .iter()
                .take(EVIDENCE_SAMPLE_LIMIT)
                .cloned()
                .collect(),
            side_effects_total: summary.side_effects.len(),
            side_effect_samples: summary
                .side_effects
                .iter()
                .take(EVIDENCE_SAMPLE_LIMIT)
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProbeFailure {
    pub(crate) probe_id: String,
    pub(crate) intent: Option<String>,
    pub(crate) path: Vec<String>,
    pub(crate) argv: Vec<String>,
    pub(crate) status: String,
    pub(crate) evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SideEffectRecord {
    pub(crate) evidence: String,
    pub(crate) probe_id: String,
    pub(crate) intent: Option<String>,
    pub(crate) command_path: Vec<String>,
    pub(crate) argv: Vec<String>,
    pub(crate) kind: String,
    pub(crate) region: String,
    pub(crate) path: String,
    pub(crate) size_bytes: Option<u64>,
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_owned))
        .collect()
}

fn status_label(status: &Value) -> String {
    match status["state"].as_str() {
        Some("exited") => match status["code"].as_i64() {
            Some(code) => format!("exited:{code}"),
            None => "exited:none".to_owned(),
        },
        Some("timed_out") => "timed_out".to_owned(),
        Some("spawn_failed") => status["error"].as_str().map_or_else(
            || "spawn_failed".to_owned(),
            |error| format!("spawn_failed:{error}"),
        ),
        Some(other) => other.to_owned(),
        None => "unknown".to_owned(),
    }
}
