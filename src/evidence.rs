use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::artifacts::EVIDENCE_JSONL;
use crate::context::RuntimeContext;
use crate::error::{CliareError, Result};
use crate::fingerprint::TargetFingerprint;
use crate::process::{OutputCapture, ProbeOutcome};
use crate::sandbox::{ProbeSandboxEvidence, SandboxMetadata, SideEffectSummary};

const SCHEMA_VERSION: &str = "cliare.evidence.v1";
pub const EVIDENCE_IN_PROGRESS_PREFIX: &str = ".evidence.jsonl.in-progress.";

#[derive(Debug, Clone, Copy)]
pub struct EventId(u64);

impl EventId {
    fn next(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }

    fn current(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub struct EvidenceWriter {
    next_event_id: EventId,
    path: PathBuf,
    final_path: PathBuf,
    file: File,
}

impl EvidenceWriter {
    pub async fn create(out_dir: &Path) -> Result<Self> {
        fs::create_dir_all(out_dir)
            .await
            .map_err(|source| CliareError::CreateArtifactDir {
                path: out_dir.to_path_buf(),
                source,
            })?;

        let final_path = out_dir.join(EVIDENCE_JSONL);
        let path = out_dir.join(format!(
            "{EVIDENCE_IN_PROGRESS_PREFIX}{}",
            std::process::id()
        ));
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .await
            .map_err(|source| CliareError::OpenEvidenceLog {
                path: path.clone(),
                source,
            })?;

        Ok(Self {
            next_event_id: EventId(1),
            path,
            final_path,
            file,
        })
    }

    pub async fn resume(out_dir: &Path, path: PathBuf, next_event_id: u64) -> Result<Self> {
        let final_path = out_dir.join(EVIDENCE_JSONL);
        let file = OpenOptions::new()
            .create(false)
            .append(true)
            .open(&path)
            .await
            .map_err(|source| CliareError::OpenEvidenceLog {
                path: path.clone(),
                source,
            })?;

        Ok(Self {
            next_event_id: EventId(next_event_id),
            path,
            final_path,
            file,
        })
    }

    pub async fn append(&mut self, kind: EvidenceKind) -> Result<String> {
        let event_id = format!("e_{:06}", self.next_event_id.next().0);
        let event = EvidenceEvent {
            schema_version: SCHEMA_VERSION,
            event_id: event_id.clone(),
            timestamp: timestamp()?,
            kind,
        };

        let mut line = serde_json::to_vec(&event).map_err(CliareError::SerializeEvidence)?;
        line.push(b'\n');
        self.file
            .write_all(&line)
            .await
            .map_err(CliareError::WriteEvidence)?;
        self.file
            .flush()
            .await
            .map_err(CliareError::WriteEvidence)?;

        Ok(event_id)
    }

    pub async fn commit(mut self) -> Result<PathBuf> {
        self.file
            .flush()
            .await
            .map_err(CliareError::WriteEvidence)?;
        drop(self.file);
        fs::rename(&self.path, &self.final_path)
            .await
            .map_err(|source| CliareError::CommitEvidence {
                path: self.final_path.clone(),
                source,
            })?;
        Ok(self.final_path)
    }

    pub fn next_event_id(&self) -> u64 {
        self.next_event_id.current()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn timestamp() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(CliareError::TimeFormat)
}

#[derive(Debug, Serialize)]
struct EvidenceEvent {
    schema_version: &'static str,
    event_id: String,
    timestamp: String,
    #[serde(flatten)]
    kind: EvidenceKind,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum EvidenceKind {
    RunStarted(RunStarted),
    ProbeScheduled(ProbeScheduled),
    ProcessCompleted(ProcessCompleted),
    RunFinished(RunFinished),
}

#[derive(Debug, Serialize)]
pub struct RunStarted {
    pub target: TargetFingerprint,
    pub artifact_dir: PathBuf,
    pub runtime_context: RuntimeContext,
    pub sandbox: SandboxMetadata,
}

#[derive(Debug, Serialize)]
pub struct ProbeScheduled {
    pub probe_id: String,
    pub argv: Vec<String>,
    pub path: Vec<String>,
    pub intent: ProbeIntent,
    pub sandbox: ProbeSandboxEvidence,
}

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

#[derive(Debug, Serialize)]
pub struct RunFinished {
    pub probes_completed: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeIntent {
    Help,
    Version,
    InvalidCommand,
    InvalidChild,
    InvalidFlag,
    OutputJson,
    OutputYaml,
    OutputTable,
    OutputPlain,
    OutputJsonHelp,
    OutputYamlHelp,
    OutputTableHelp,
    OutputPlainHelp,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProcessStatus {
    Exited { code: Option<i32> },
    TimedOut,
    SpawnFailed { error: String },
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
