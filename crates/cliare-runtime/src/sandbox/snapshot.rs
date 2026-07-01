use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use cliare_core::error::{CliareError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

use super::{ProcessSandbox, SandboxRegionRoot, SnapshotHashMode, SnapshotLimits};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxRegion {
    Home,
    Workdir,
    XdgConfig,
    XdgCache,
    XdgData,
    Tmp,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct FileChange {
    pub kind: FileChangeKind,
    pub region: SandboxRegion,
    pub path: PathBuf,
    pub size_bytes: Option<u64>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct SideEffectSummary {
    pub created: usize,
    pub modified: usize,
    pub deleted: usize,
    pub total: usize,
    pub truncated: bool,
    pub truncation_reason: Option<String>,
    pub changes: Vec<FileChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideEffectSnapshot {
    files: BTreeMap<PathBuf, FileSnapshot>,
    truncated: bool,
    truncation_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSnapshot {
    region: SandboxRegion,
    size_bytes: u64,
    modified_unix_nanos: Option<u128>,
    sha256: Option<String>,
}

impl ProcessSandbox {
    pub async fn snapshot(&self) -> Result<SideEffectSnapshot> {
        let mut files = BTreeMap::new();
        let mut budget = SnapshotBudget::new(self.snapshot_limits);

        for region_root in &self.regions {
            scan_region(&self.root, region_root, &mut files, &mut budget).await?;
            if budget.truncated() {
                break;
            }
        }

        Ok(SideEffectSnapshot {
            files,
            truncated: budget.truncated(),
            truncation_reason: budget.truncation_reason().map(str::to_owned),
        })
    }
}

impl SideEffectSnapshot {
    pub fn diff(&self, after: &Self) -> SideEffectSummary {
        let mut changes = Vec::new();

        for (path, before_file) in &self.files {
            match after.files.get(path) {
                Some(after_file) if after_file == before_file => {}
                Some(after_file) => changes.push(FileChange {
                    kind: FileChangeKind::Modified,
                    region: after_file.region,
                    path: path.clone(),
                    size_bytes: Some(after_file.size_bytes),
                    sha256: after_file.sha256.clone(),
                }),
                None => changes.push(FileChange {
                    kind: FileChangeKind::Deleted,
                    region: before_file.region,
                    path: path.clone(),
                    size_bytes: None,
                    sha256: None,
                }),
            }
        }

        for (path, after_file) in &after.files {
            if self.files.contains_key(path) {
                continue;
            }
            changes.push(FileChange {
                kind: FileChangeKind::Created,
                region: after_file.region,
                path: path.clone(),
                size_bytes: Some(after_file.size_bytes),
                sha256: after_file.sha256.clone(),
            });
        }

        let created = changes
            .iter()
            .filter(|change| change.kind == FileChangeKind::Created)
            .count();
        let modified = changes
            .iter()
            .filter(|change| change.kind == FileChangeKind::Modified)
            .count();
        let deleted = changes
            .iter()
            .filter(|change| change.kind == FileChangeKind::Deleted)
            .count();

        SideEffectSummary {
            created,
            modified,
            deleted,
            total: changes.len(),
            truncated: self.truncated || after.truncated,
            truncation_reason: self
                .truncation_reason
                .clone()
                .or_else(|| after.truncation_reason.clone()),
            changes,
        }
    }
}

#[derive(Debug)]
struct SnapshotBudget {
    limits: SnapshotLimits,
    files: usize,
    directories: usize,
    hash_bytes: u64,
    truncation_reason: Option<&'static str>,
}

impl SnapshotBudget {
    fn new(limits: SnapshotLimits) -> Self {
        Self {
            limits,
            files: 0,
            directories: 0,
            hash_bytes: 0,
            truncation_reason: None,
        }
    }

    fn record_directory(&mut self) -> bool {
        self.directories = self.directories.saturating_add(1);
        if self.directories > self.limits.max_directories {
            self.truncate("directory_budget_exhausted");
            false
        } else {
            true
        }
    }

    fn record_file(&mut self) -> bool {
        self.files = self.files.saturating_add(1);
        if self.files > self.limits.max_files {
            self.truncate("file_budget_exhausted");
            false
        } else {
            true
        }
    }

    fn remaining_hash_bytes(&self) -> u64 {
        self.limits.max_hash_bytes.saturating_sub(self.hash_bytes)
    }

    fn consume_hash_bytes(&mut self, bytes: u64) {
        self.hash_bytes = self.hash_bytes.saturating_add(bytes);
    }

    fn truncate(&mut self, reason: &'static str) {
        if self.truncation_reason.is_none() {
            self.truncation_reason = Some(reason);
        }
    }

    fn truncated(&self) -> bool {
        self.truncation_reason.is_some()
    }

    fn truncation_reason(&self) -> Option<&'static str> {
        self.truncation_reason
    }
}

async fn scan_region(
    sandbox_root: &Path,
    region_root: &SandboxRegionRoot,
    files: &mut BTreeMap<PathBuf, FileSnapshot>,
    budget: &mut SnapshotBudget,
) -> Result<()> {
    let mut pending = vec![region_root.path.clone()];

    while let Some(dir) = pending.pop() {
        if !budget.record_directory() {
            return Ok(());
        }
        let mut entries = match fs::read_dir(&dir).await {
            Ok(entries) => entries,
            Err(source) if source.kind() == ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(CliareError::ReadSandboxDir {
                    path: dir.clone(),
                    source,
                });
            }
        };

        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(source) if source.kind() == ErrorKind::NotFound => continue,
                Err(source) => {
                    return Err(CliareError::ReadSandboxDir {
                        path: dir.clone(),
                        source,
                    });
                }
            };
            let path = entry.path();
            if budget.truncated() {
                return Ok(());
            }
            let metadata = match entry.metadata().await {
                Ok(metadata) => metadata,
                Err(source) if source.kind() == ErrorKind::NotFound => continue,
                Err(source) => {
                    return Err(CliareError::ReadSandboxMetadata {
                        path: path.clone(),
                        source,
                    });
                }
            };

            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                if !budget.record_file() {
                    return Ok(());
                }
                let sha256 = match region_root.hash_mode {
                    SnapshotHashMode::Content => {
                        match sha256_file(&path, budget.remaining_hash_bytes()).await? {
                            FileHash::Complete { sha256, bytes_read } => {
                                budget.consume_hash_bytes(bytes_read);
                                sha256
                            }
                            FileHash::Truncated => {
                                budget.truncate("hash_byte_budget_exhausted");
                                None
                            }
                        }
                    }
                    SnapshotHashMode::Metadata => None,
                };
                let relative_path = path
                    .strip_prefix(sandbox_root)
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|_| path.clone());
                files.insert(
                    relative_path,
                    FileSnapshot {
                        region: region_root.region,
                        size_bytes: metadata.len(),
                        modified_unix_nanos: modified_unix_nanos(&metadata),
                        sha256,
                    },
                );
                if budget.truncated() {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

fn modified_unix_nanos(metadata: &std::fs::Metadata) -> Option<u128> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

enum FileHash {
    Complete {
        sha256: Option<String>,
        bytes_read: u64,
    },
    Truncated,
}

async fn sha256_file(path: &Path, max_bytes: u64) -> Result<FileHash> {
    if max_bytes == 0 {
        return Ok(FileHash::Truncated);
    }
    let mut file = match File::open(path).await {
        Ok(file) => file,
        Err(source) if source.kind() == ErrorKind::NotFound => {
            return Ok(FileHash::Complete {
                sha256: None,
                bytes_read: 0,
            });
        }
        Err(source) => {
            return Err(CliareError::ReadSandboxFile {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut bytes_read_total = 0_u64;

    loop {
        let bytes_read =
            file.read(&mut buffer)
                .await
                .map_err(|source| CliareError::ReadSandboxFile {
                    path: path.to_path_buf(),
                    source,
                })?;
        if bytes_read == 0 {
            break;
        }
        let bytes_read = bytes_read as u64;
        if bytes_read_total.saturating_add(bytes_read) > max_bytes {
            return Ok(FileHash::Truncated);
        }
        bytes_read_total = bytes_read_total.saturating_add(bytes_read);
        hasher.update(&buffer[..bytes_read as usize]);
    }

    Ok(FileHash::Complete {
        sha256: Some(format!("{:x}", hasher.finalize())),
        bytes_read: bytes_read_total,
    })
}
