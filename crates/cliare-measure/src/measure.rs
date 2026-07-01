mod bootstrap;
mod cache;
mod checkpoint;
mod profile;
mod progress;
mod run;
mod summary;
#[cfg(test)]
mod tests;
mod traversal;

const MEASUREMENT_CACHE_SCHEMA_VERSION: &str = "cliare.measure-cache.v1";
const MEASUREMENT_CHECKPOINT_SCHEMA_VERSION: &str = "cliare.measure-checkpoint.v1";
const MEASUREMENT_ENGINE: &str = "cliare-measure-v0";
const MEASUREMENT_CHECKPOINT_JSON: &str = "measure-checkpoint.json";

pub use progress::new_measure_job_id;
pub use run::measure;
pub use summary::{MeasurementFacts, MeasurementSummary};

#[cfg(test)]
use crate::artifacts::REQUIRED_MEASUREMENT_FILES;
#[cfg(test)]
use bootstrap::{bootstrap_probes, invalid_token_seed};
#[cfg(test)]
use cache::{artifact_digests, cleanup_abandoned_in_progress_files, remove_stale_cache_manifest};
#[cfg(test)]
use checkpoint::{MeasurementCheckpoint, read_resume_checkpoint};
#[cfg(test)]
use profile::ProbeProfile;
#[cfg(test)]
use progress::progress_percent;
