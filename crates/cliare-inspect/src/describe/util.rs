use std::path::Path;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::fs;

use cliare_core::error::{CliareError, Result};

use super::model::ArtifactMap;

pub(super) async fn ensure_directory(path: &Path) -> Result<()> {
    let metadata =
        fs::metadata(path)
            .await
            .map_err(|source| CliareError::ReadArtifactDirectory {
                path: path.to_path_buf(),
                source,
            })?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(CliareError::ArtifactPathNotDirectory {
            path: path.to_path_buf(),
        })
    }
}

pub(super) async fn write_json(path: &Path, map: &ArtifactMap) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(map).map_err(CliareError::SerializeArtifactMap)?;
    fs::write(path, bytes)
        .await
        .map_err(|source| CliareError::WriteArtifactMap {
            path: path.to_path_buf(),
            source,
        })
}

pub(super) async fn write_markdown(path: &Path, markdown: &str) -> Result<()> {
    fs::write(path, markdown.as_bytes())
        .await
        .map_err(|source| CliareError::WriteArtifactMap {
            path: path.to_path_buf(),
            source,
        })
}

pub(super) fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(super) fn timestamp() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(CliareError::TimeFormat)
}
