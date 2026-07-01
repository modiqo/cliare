use std::path::{Path, PathBuf};

use cliare_core::error::{CliareError, Result};
use serde::de::DeserializeOwned;

use super::model::{ShapeArtifact, ShapeQualityReport, ShapeTruth};

pub(super) async fn read_shape(path: &Path) -> Result<ShapeArtifact> {
    read_json(path).await
}

pub(super) async fn read_truth(path: &Path) -> Result<ShapeTruth> {
    read_json(path).await
}

async fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|source| CliareError::ReadEvalArtifact {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseEvalArtifact {
        path: path.to_path_buf(),
        source,
    })
}

pub(super) async fn write_json_report(
    out_dir: &Path,
    report: &ShapeQualityReport,
) -> Result<PathBuf> {
    let path = out_dir.join("shape-quality.json");
    let bytes = serde_json::to_vec_pretty(report).map_err(CliareError::SerializeEvalArtifact)?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|source| CliareError::WriteEvalArtifact {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}

pub(super) async fn write_markdown_report(out_dir: &Path, markdown: &str) -> Result<PathBuf> {
    let path = out_dir.join("shape-quality.md");
    tokio::fs::write(&path, markdown)
        .await
        .map_err(|source| CliareError::WriteEvalArtifact {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}
