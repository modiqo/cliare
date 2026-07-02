use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;

use crate::artifacts::{MeasurementArtifactPaths, REQUIRED_MEASUREMENT_FILES};
use crate::error::{CliareError, Result};
use crate::evidence::EVIDENCE_IN_PROGRESS_PREFIX;
use crate::fingerprint::TargetFingerprint;
use crate::report;

use super::profile::ProbeProfile;
use super::summary::{MeasurementFacts, MeasurementSummary};
use super::{MEASUREMENT_CACHE_SCHEMA_VERSION, MEASUREMENT_ENGINE};

#[derive(Debug, Deserialize, Serialize)]
struct MeasurementCacheManifest {
    schema_version: String,
    cliare_version: String,
    engine: String,
    #[serde(default)]
    run_id: String,
    target: TargetFingerprint,
    profile: ProbeProfile,
    #[serde(default)]
    artifact_digests: Vec<ArtifactDigest>,
    summary: MeasurementFacts,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub(super) struct ArtifactDigest {
    pub(super) path: String,
    pub(super) sha256: String,
    pub(super) size_bytes: u64,
}

pub(super) async fn cached_summary(
    out_dir: &std::path::Path,
    target: &TargetFingerprint,
    profile: &ProbeProfile,
) -> Result<Option<MeasurementSummary>> {
    let path = out_dir.join("measure-cache.json");
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliareError::ReadMeasurementCache { path, source });
        }
    };
    let manifest: MeasurementCacheManifest =
        serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseMeasurementCache {
            path: path.clone(),
            source,
        })?;

    if !manifest.matches(target, profile)
        || !artifacts_exist(out_dir).await?
        || !manifest.artifact_digests_match(out_dir).await?
    {
        return Ok(None);
    }

    Ok(Some(manifest.into_summary(out_dir)))
}

pub(super) async fn remove_stale_cache_manifest(out_dir: &std::path::Path) -> Result<()> {
    let path = out_dir.join("measure-cache.json");
    match fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliareError::RemoveMeasurementCache { path, source }),
    }
}

pub(super) async fn cleanup_abandoned_in_progress_files(out_dir: &std::path::Path) -> Result<()> {
    let mut entries = match fs::read_dir(out_dir).await {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(CliareError::CleanupInProgressArtifact {
                path: out_dir.to_path_buf(),
                source,
            });
        }
    };

    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(source) => {
                return Err(CliareError::CleanupInProgressArtifact {
                    path: out_dir.to_path_buf(),
                    source,
                });
            }
        };
        let path = entry.path();
        let is_in_progress = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(EVIDENCE_IN_PROGRESS_PREFIX));
        if !is_in_progress {
            continue;
        }
        fs::remove_file(&path)
            .await
            .map_err(|source| CliareError::CleanupInProgressArtifact {
                path: path.clone(),
                source,
            })?;
    }

    Ok(())
}

impl MeasurementCacheManifest {
    fn matches(&self, target: &TargetFingerprint, profile: &ProbeProfile) -> bool {
        self.schema_version == MEASUREMENT_CACHE_SCHEMA_VERSION
            && self.cliare_version == env!("CARGO_PKG_VERSION")
            && self.engine == MEASUREMENT_ENGINE
            && !self.run_id.trim().is_empty()
            && &self.target == target
            && &self.profile == profile
    }

    pub(super) async fn artifact_digests_match(&self, out_dir: &std::path::Path) -> Result<bool> {
        if self.artifact_digests.is_empty() {
            return Ok(false);
        }
        Ok(self.artifact_digests == artifact_digests(out_dir).await?)
    }

    fn into_summary(self, out_dir: &std::path::Path) -> MeasurementSummary {
        let paths = MeasurementArtifactPaths::from_dir(out_dir);
        MeasurementSummary {
            target: self.target,
            job_id: None,
            job_log_path: None,
            evidence_path: paths.evidence,
            shape_path: paths.shape,
            command_index_json_path: paths.command_index_json,
            command_index_markdown_path: paths.command_index_markdown,
            scorecard_path: paths.scorecard,
            report_path: paths.report,
            ci_summary_path: paths.ci_summary,
            sarif_path: paths.sarif,
            junit_path: paths.junit,
            issues_markdown_path: paths.issues_markdown,
            issues_json_path: paths.issues_json,
            persona_report_count: report::Persona::all().len(),
            readme_path: paths.readme,
            agent_skill_path: paths.agent_skill,
            condition_dictionary_path: paths.condition_dictionary,
            facts: self.summary,
            cache_hit: true,
            runtime_context: self.profile.runtime_context,
            suite_root_path: out_dir.to_path_buf(),
            runtime_context_path: Some(paths.runtime_context),
            context_suite_path: None,
            context_compare_path: None,
        }
    }
}

pub(super) async fn artifacts_exist(out_dir: &std::path::Path) -> Result<bool> {
    for name in REQUIRED_MEASUREMENT_FILES {
        let path = out_dir.join(name);
        match fs::metadata(&path).await {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Ok(false),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(source) => {
                return Err(CliareError::ReadMeasurementCache { path, source });
            }
        }
    }
    Ok(true)
}

pub(super) async fn write_cache_manifest(
    out_dir: &std::path::Path,
    summary: &MeasurementSummary,
    profile: ProbeProfile,
    run_id: &str,
) -> Result<()> {
    let path = out_dir.join("measure-cache.json");
    let manifest = MeasurementCacheManifest {
        schema_version: MEASUREMENT_CACHE_SCHEMA_VERSION.to_owned(),
        cliare_version: env!("CARGO_PKG_VERSION").to_owned(),
        engine: MEASUREMENT_ENGINE.to_owned(),
        run_id: run_id.to_owned(),
        target: summary.target.clone(),
        profile,
        artifact_digests: artifact_digests(out_dir).await?,
        summary: summary.facts.clone(),
    };
    let bytes =
        serde_json::to_vec_pretty(&manifest).map_err(CliareError::SerializeMeasurementCache)?;
    crate::artifacts::write_atomic(&path, &bytes)
        .await
        .map_err(|source| CliareError::WriteMeasurementCache { path, source })
}

pub(super) async fn artifact_digests(out_dir: &std::path::Path) -> Result<Vec<ArtifactDigest>> {
    let mut digests = Vec::with_capacity(REQUIRED_MEASUREMENT_FILES.len());
    for name in REQUIRED_MEASUREMENT_FILES {
        let path = out_dir.join(name);
        let bytes = fs::read(&path)
            .await
            .map_err(|source| CliareError::ReadMeasurementCache {
                path: path.clone(),
                source,
            })?;
        digests.push(ArtifactDigest {
            path: (*name).to_owned(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            size_bytes: bytes.len() as u64,
        });
    }
    Ok(digests)
}
