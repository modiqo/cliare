use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::cli::{MeasureArgs, TraversalProfile};
use crate::error::{CliareError, Result};

use super::CORPUS_SCHEMA_VERSION;

pub(super) async fn read_corpus(path: &Path) -> Result<BenchmarkCorpus> {
    let bytes = fs::read(path)
        .await
        .map_err(|source| CliareError::ReadBenchmarkManifest {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseBenchmarkManifest {
        path: path.to_path_buf(),
        source,
    })
}

pub(super) fn validate_corpus(corpus: &BenchmarkCorpus) -> Result<()> {
    if corpus.schema_version != CORPUS_SCHEMA_VERSION {
        return Err(CliareError::UnsupportedBenchmarkSchema {
            schema_version: corpus.schema_version.clone(),
        });
    }
    validate_positive(
        corpus.defaults.target_concurrency,
        "defaults.target_concurrency",
    )?;
    validate_positive(corpus.defaults.concurrency, "defaults.concurrency")?;
    for target in &corpus.targets {
        validate_positive(target.concurrency, "targets.concurrency")?;
        if let Some(band) = &target.expected_score
            && !(band.min.is_finite()
                && band.max.is_finite()
                && (0.0..=100.0).contains(&band.min)
                && (0.0..=100.0).contains(&band.max)
                && band.min <= band.max)
        {
            return Err(CliareError::InvalidBenchmarkScoreBand {
                target_id: target.id.clone(),
                min: band.min,
                max: band.max,
            });
        }
    }
    Ok(())
}

fn validate_positive(value: Option<usize>, field: &'static str) -> Result<()> {
    if let Some(0) = value {
        return Err(CliareError::InvalidBenchmarkPositiveInteger { field, value: 0 });
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct BenchmarkCorpus {
    pub(super) schema_version: String,
    pub(super) name: String,
    #[serde(default)]
    pub(super) defaults: BenchmarkDefaults,
    pub(super) targets: Vec<BenchmarkTarget>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct BenchmarkDefaults {
    pub(super) target_concurrency: Option<usize>,
    pub(super) profile: Option<TraversalProfile>,
    pub(super) max_depth: Option<usize>,
    pub(super) max_probes: Option<usize>,
    pub(super) min_expected_value: Option<u16>,
    pub(super) concurrency: Option<usize>,
    pub(super) timeout_ms: Option<u64>,
    pub(super) output_limit_bytes: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct BenchmarkTarget {
    pub(super) id: String,
    pub(super) target: PathBuf,
    #[serde(default = "default_required")]
    pub(super) required: bool,
    #[serde(default)]
    pub(super) tags: Vec<String>,
    pub(super) profile: Option<TraversalProfile>,
    pub(super) max_depth: Option<usize>,
    pub(super) max_probes: Option<usize>,
    pub(super) min_expected_value: Option<u16>,
    pub(super) concurrency: Option<usize>,
    pub(super) timeout_ms: Option<u64>,
    pub(super) output_limit_bytes: Option<usize>,
    pub(super) expected_score: Option<ScoreBand>,
    pub(super) max_duration_ms: Option<u128>,
}

impl BenchmarkTarget {
    pub(super) fn measure_args(
        &self,
        target: PathBuf,
        out: PathBuf,
        defaults: &BenchmarkDefaults,
        refresh: bool,
    ) -> MeasureArgs {
        let profile = self
            .profile
            .or(defaults.profile)
            .unwrap_or(TraversalProfile::Quick);
        MeasureArgs {
            target,
            out,
            timeout_ms: self.timeout_ms.or(defaults.timeout_ms).unwrap_or(5_000),
            output_limit_bytes: self
                .output_limit_bytes
                .or(defaults.output_limit_bytes)
                .unwrap_or(1_048_576),
            profile,
            execution_mode: crate::sandbox::SandboxProfile::Isolated,
            max_depth: self.max_depth.or(defaults.max_depth),
            max_probes: self.max_probes.or(defaults.max_probes),
            min_expected_value: self.min_expected_value.or(defaults.min_expected_value),
            concurrency: self.concurrency.or(defaults.concurrency),
            snapshot_max_files: None,
            snapshot_max_directories: None,
            snapshot_max_hash_bytes: None,
            context: None,
            context_name: None,
            auth_state: None,
            local_context_state: None,
            fixture_state: None,
            network_state: None,
            runtime_dependency_state: None,
            context_workdir: None,
            refresh,
            detach: false,
            detached_worker: false,
            job_id: None,
        }
    }
}

fn default_required() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct ScoreBand {
    pub(super) min: f64,
    pub(super) max: f64,
}
