use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{CliareError, Result};
use crate::fingerprint::TargetFingerprint;
use crate::observation::ShapeObservation;
use crate::process::ProbeSpec;

use super::profile::ProbeProfile;
use super::{
    MEASUREMENT_CHECKPOINT_JSON, MEASUREMENT_CHECKPOINT_SCHEMA_VERSION, MEASUREMENT_ENGINE,
};

#[derive(Debug, Clone, Default)]
pub(super) struct TraversalResume {
    pub(super) completed: Vec<CheckpointObservation>,
    pub(super) probes_scheduled: usize,
    pub(super) probes_completed: usize,
    pub(super) rounds: usize,
}

impl TraversalResume {
    pub(super) fn observations(&self) -> Vec<ShapeObservation> {
        self.completed
            .iter()
            .map(|entry| entry.observation.clone())
            .collect()
    }

    pub(super) fn completed_probes(&self) -> impl Iterator<Item = ProbeSpec> + '_ {
        self.completed.iter().map(|entry| entry.probe.clone())
    }
}

impl From<MeasurementCheckpoint> for TraversalResume {
    fn from(checkpoint: MeasurementCheckpoint) -> Self {
        Self {
            completed: checkpoint.completed,
            probes_scheduled: checkpoint.probes_scheduled,
            probes_completed: checkpoint.probes_completed,
            rounds: checkpoint.rounds,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct CheckpointWriter {
    pub(super) path: PathBuf,
    pub(super) target: TargetFingerprint,
    pub(super) profile: ProbeProfile,
    pub(super) evidence_path: PathBuf,
}

impl CheckpointWriter {
    pub(super) async fn write(
        &self,
        next_event_id: u64,
        completed: &[CheckpointObservation],
        probes_scheduled: usize,
        probes_completed: usize,
        rounds: usize,
    ) -> Result<()> {
        let checkpoint = MeasurementCheckpoint {
            schema_version: MEASUREMENT_CHECKPOINT_SCHEMA_VERSION.to_owned(),
            cliare_version: env!("CARGO_PKG_VERSION").to_owned(),
            engine: MEASUREMENT_ENGINE.to_owned(),
            target: self.target.clone(),
            profile: self.profile.clone(),
            evidence_path: self.evidence_path.clone(),
            next_event_id,
            probes_scheduled,
            probes_completed,
            rounds,
            completed: completed.to_vec(),
        };
        let bytes = serde_json::to_vec_pretty(&checkpoint)
            .map_err(CliareError::SerializeMeasurementCheckpoint)?;
        crate::artifacts::write_atomic(&self.path, &bytes)
            .await
            .map_err(|source| CliareError::WriteMeasurementCheckpoint {
                path: self.path.clone(),
                source,
            })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct MeasurementCheckpoint {
    pub(super) schema_version: String,
    pub(super) cliare_version: String,
    pub(super) engine: String,
    pub(super) target: TargetFingerprint,
    pub(super) profile: ProbeProfile,
    pub(super) evidence_path: PathBuf,
    pub(super) next_event_id: u64,
    pub(super) probes_scheduled: usize,
    pub(super) probes_completed: usize,
    pub(super) rounds: usize,
    pub(super) completed: Vec<CheckpointObservation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct CheckpointObservation {
    pub(super) probe: ProbeSpec,
    pub(super) observation: ShapeObservation,
}

pub(super) async fn read_resume_checkpoint(
    out_dir: &Path,
    target: &TargetFingerprint,
    profile: &ProbeProfile,
) -> Result<Option<MeasurementCheckpoint>> {
    let path = out_dir.join(MEASUREMENT_CHECKPOINT_JSON);
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(CliareError::ReadMeasurementCheckpoint { path, source }),
    };
    let checkpoint: MeasurementCheckpoint = serde_json::from_slice(&bytes).map_err(|source| {
        CliareError::ParseMeasurementCheckpoint {
            path: path.clone(),
            source,
        }
    })?;

    if checkpoint.schema_version != MEASUREMENT_CHECKPOINT_SCHEMA_VERSION
        || checkpoint.engine != MEASUREMENT_ENGINE
        || checkpoint.cliare_version != env!("CARGO_PKG_VERSION")
        || &checkpoint.target != target
        || &checkpoint.profile != profile
        || checkpoint.completed.len() != checkpoint.probes_completed
    {
        return Ok(None);
    }

    let evidence_path = checkpoint.evidence_path.clone();
    match fs::metadata(&evidence_path).await {
        Ok(metadata) if metadata.is_file() => Ok(Some(checkpoint)),
        Ok(_) => Ok(None),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CliareError::ReadMeasurementCheckpoint {
            path: evidence_path,
            source,
        }),
    }
}

pub(super) async fn remove_measurement_checkpoint(out_dir: &Path) -> Result<()> {
    let path = out_dir.join(MEASUREMENT_CHECKPOINT_JSON);
    match fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliareError::RemoveMeasurementCheckpoint { path, source }),
    }
}
