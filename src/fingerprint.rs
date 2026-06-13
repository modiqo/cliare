use std::env;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

use crate::error::{CliareError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TargetFingerprint {
    pub requested: PathBuf,
    pub resolved: PathBuf,
    pub binary_sha256: String,
    pub size_bytes: u64,
}

pub async fn fingerprint_target(requested: &Path) -> Result<TargetFingerprint> {
    let resolved = resolve_target(requested)?;
    let metadata = fs::metadata(&resolved)
        .await
        .map_err(|source| CliareError::Fingerprint {
            path: resolved.clone(),
            source,
        })?;

    if !metadata.is_file() {
        return Err(CliareError::TargetNotFile(resolved));
    }

    let binary_sha256 = sha256_file(&resolved).await?;

    Ok(TargetFingerprint {
        requested: requested.to_path_buf(),
        resolved,
        binary_sha256,
        size_bytes: metadata.len(),
    })
}

fn resolve_target(requested: &Path) -> Result<PathBuf> {
    if requested.components().count() > 1 || requested.is_absolute() {
        return Ok(requested.to_path_buf());
    }

    let Some(path_var) = env::var_os("PATH") else {
        return Err(CliareError::TargetNotFound(requested.to_path_buf()));
    };

    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(requested);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(CliareError::TargetNotFound(requested.to_path_buf()))
}

async fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .await
        .map_err(|source| CliareError::Fingerprint {
            path: path.to_path_buf(),
            source,
        })?;

    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let bytes_read =
            file.read(&mut buffer)
                .await
                .map_err(|source| CliareError::Fingerprint {
                    path: path.to_path_buf(),
                    source,
                })?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
