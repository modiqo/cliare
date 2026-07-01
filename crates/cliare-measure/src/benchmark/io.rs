use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::error::{CliareError, Result};

pub(super) async fn write_atomic(
    path: &Path,
    bytes: Vec<u8>,
    error: impl Fn(PathBuf, std::io::Error) -> CliareError,
) -> Result<()> {
    let temp_path = atomic_temp_path(path);
    fs::write(&temp_path, bytes)
        .await
        .map_err(|source| error(temp_path.clone(), source))?;
    fs::rename(&temp_path, path)
        .await
        .map_err(|source| error(path.to_path_buf(), source))?;
    Ok(())
}

fn atomic_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("benchmark");
    path.with_file_name(format!("{file_name}.tmp.{}", std::process::id()))
}

pub(super) struct BenchmarkOutputLock {
    path: PathBuf,
}

impl BenchmarkOutputLock {
    pub(super) async fn acquire(out_dir: &Path) -> Result<Self> {
        let path = out_dir.join(".benchmark.lock");
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => file,
            Err(source) if source.kind() == ErrorKind::AlreadyExists => {
                return Err(CliareError::BenchmarkOutputLocked { path });
            }
            Err(source) => {
                return Err(CliareError::AcquireBenchmarkLock { path, source });
            }
        };
        let contents = format!("pid={}\n", std::process::id());
        file.write_all(contents.as_bytes())
            .await
            .map_err(|source| CliareError::AcquireBenchmarkLock {
                path: path.clone(),
                source,
            })?;
        Ok(Self { path })
    }
}

impl Drop for BenchmarkOutputLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
