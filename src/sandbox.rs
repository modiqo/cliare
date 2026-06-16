use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

#[cfg(not(test))]
pub const DEFAULT_SNAPSHOT_MAX_FILES: usize = 10_000;
#[cfg(test)]
pub const DEFAULT_SNAPSHOT_MAX_FILES: usize = 32;
#[cfg(not(test))]
pub const DEFAULT_SNAPSHOT_MAX_DIRS: usize = 2_000;
#[cfg(test)]
pub const DEFAULT_SNAPSHOT_MAX_DIRS: usize = 32;
pub const DEFAULT_SNAPSHOT_MAX_HASH_BYTES: u64 = 64 * 1024 * 1024;

use crate::error::{CliareError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct SnapshotLimits {
    pub max_files: usize,
    pub max_directories: usize,
    pub max_hash_bytes: u64,
}

impl SnapshotLimits {
    pub fn new(max_files: usize, max_directories: usize, max_hash_bytes: u64) -> Self {
        Self {
            max_files,
            max_directories,
            max_hash_bytes,
        }
    }
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            max_files: DEFAULT_SNAPSHOT_MAX_FILES,
            max_directories: DEFAULT_SNAPSHOT_MAX_DIRS,
            max_hash_bytes: DEFAULT_SNAPSHOT_MAX_HASH_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum SandboxProfile {
    Isolated,
    Host,
}

impl SandboxProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
            Self::Host => "host",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SandboxMetadata {
    pub profile: SandboxProfile,
    pub root: PathBuf,
    pub home: PathBuf,
    pub workdir: PathBuf,
    pub xdg_config_home: PathBuf,
    pub xdg_cache_home: PathBuf,
    pub xdg_data_home: PathBuf,
    pub tmp: PathBuf,
    pub env_policy: EnvPolicy,
    pub env_keys: Vec<String>,
    pub snapshot_limits: SnapshotLimits,
    pub hostile_binary_containment: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvPolicy {
    ClearedWithAllowlist,
    Inherited,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeSandboxEvidence {
    pub profile: SandboxProfile,
    pub cwd: PathBuf,
    pub env_policy: EnvPolicy,
    pub env_keys: Vec<String>,
    pub snapshot_limits: SnapshotLimits,
    pub hostile_binary_containment: bool,
}

#[derive(Debug, Clone)]
pub struct ProcessSandbox {
    pub root: PathBuf,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    snapshot_limits: SnapshotLimits,
    regions: Vec<SandboxRegionRoot>,
}

#[derive(Debug, Clone)]
pub struct Sandbox {
    metadata: SandboxMetadata,
    env: BTreeMap<String, String>,
    provided_workdir: Option<PathBuf>,
    snapshot_limits: SnapshotLimits,
}

impl Sandbox {
    pub async fn create(out_dir: &Path) -> Result<Self> {
        Self::create_with_workdir(out_dir, None).await
    }

    pub async fn create_with_workdir(out_dir: &Path, workdir: Option<&Path>) -> Result<Self> {
        Self::create_with_profile(out_dir, workdir, SandboxProfile::Isolated).await
    }

    pub async fn create_with_profile(
        out_dir: &Path,
        workdir: Option<&Path>,
        profile: SandboxProfile,
    ) -> Result<Self> {
        Self::create_with_profile_and_limits(out_dir, workdir, profile, SnapshotLimits::default())
            .await
    }

    pub async fn create_with_profile_and_limits(
        out_dir: &Path,
        workdir: Option<&Path>,
        profile: SandboxProfile,
        snapshot_limits: SnapshotLimits,
    ) -> Result<Self> {
        match profile {
            SandboxProfile::Isolated => {
                Self::create_isolated(out_dir, workdir, snapshot_limits).await
            }
            SandboxProfile::Host => Self::create_host(out_dir, workdir, snapshot_limits).await,
        }
    }

    async fn create_isolated(
        out_dir: &Path,
        workdir: Option<&Path>,
        snapshot_limits: SnapshotLimits,
    ) -> Result<Self> {
        let root = out_dir.join("sandbox");
        if fs::metadata(&root).await.is_ok() {
            fs::remove_dir_all(&root)
                .await
                .map_err(|source| CliareError::ClearSandboxDir {
                    path: root.clone(),
                    source,
                })?;
        }
        let provided_workdir = match workdir {
            Some(path) => Some(resolve_workdir(path).await?),
            None => None,
        };
        let paths = SandboxPaths::with_workdir(root.clone(), provided_workdir.clone());
        create_execution_dirs(&paths).await?;

        let env = sandbox_env(&paths);

        let env_keys = env.keys().cloned().collect();
        let metadata = SandboxMetadata {
            profile: SandboxProfile::Isolated,
            root,
            home: paths.home,
            workdir: paths.workdir,
            xdg_config_home: paths.xdg_config_home,
            xdg_cache_home: paths.xdg_cache_home,
            xdg_data_home: paths.xdg_data_home,
            tmp: paths.tmp,
            env_policy: EnvPolicy::ClearedWithAllowlist,
            env_keys,
            snapshot_limits,
            hostile_binary_containment: false,
        };

        Ok(Self {
            metadata,
            env,
            provided_workdir,
            snapshot_limits,
        })
    }

    async fn create_host(
        out_dir: &Path,
        workdir: Option<&Path>,
        snapshot_limits: SnapshotLimits,
    ) -> Result<Self> {
        let provided_workdir = match workdir {
            Some(path) => Some(resolve_workdir(path).await?),
            None => None,
        };
        let cwd = match &provided_workdir {
            Some(path) => path.clone(),
            None => std::env::current_dir().map_err(CliareError::CurrentDirectory)?,
        };
        let env = host_env(&cwd);
        let root = out_dir.join("host-execution");
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| cwd.clone());
        let xdg_config_home = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let xdg_cache_home = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cache"));
        let xdg_data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));
        let tmp = std::env::temp_dir();
        let env_keys = env.keys().cloned().collect();
        let metadata = SandboxMetadata {
            profile: SandboxProfile::Host,
            root,
            home,
            workdir: cwd,
            xdg_config_home,
            xdg_cache_home,
            xdg_data_home,
            tmp,
            env_policy: EnvPolicy::Inherited,
            env_keys,
            snapshot_limits,
            hostile_binary_containment: false,
        };

        Ok(Self {
            metadata,
            env,
            provided_workdir,
            snapshot_limits,
        })
    }

    pub fn metadata(&self) -> &SandboxMetadata {
        &self.metadata
    }

    pub fn execution(&self) -> ProcessSandbox {
        process_sandbox(
            SandboxPaths::from_metadata(&self.metadata),
            self.env.clone(),
            self.snapshot_limits,
        )
    }

    pub async fn execution_for_probe(&self, probe_id: &str) -> Result<ProcessSandbox> {
        if self.metadata.profile == SandboxProfile::Host {
            return Ok(process_host(
                self.metadata.workdir.clone(),
                self.env.clone(),
                self.snapshot_limits,
            ));
        }
        let paths = SandboxPaths::with_workdir(
            self.metadata.root.join("probes").join(probe_id),
            self.provided_workdir.clone(),
        );
        create_execution_dirs(&paths).await?;
        Ok(process_sandbox(
            paths.clone(),
            sandbox_env(&paths),
            self.snapshot_limits,
        ))
    }

    pub fn probe_evidence(&self) -> ProbeSandboxEvidence {
        ProbeSandboxEvidence {
            profile: self.metadata.profile,
            cwd: self.metadata.workdir.clone(),
            env_policy: self.metadata.env_policy,
            env_keys: self.metadata.env_keys.clone(),
            snapshot_limits: self.snapshot_limits,
            hostile_binary_containment: self.metadata.hostile_binary_containment,
        }
    }

    pub fn probe_evidence_for(&self, execution: &ProcessSandbox) -> ProbeSandboxEvidence {
        ProbeSandboxEvidence {
            profile: self.metadata.profile,
            cwd: execution.cwd.clone(),
            env_policy: self.metadata.env_policy,
            env_keys: self.metadata.env_keys.clone(),
            snapshot_limits: self.snapshot_limits,
            hostile_binary_containment: self.metadata.hostile_binary_containment,
        }
    }
}

#[derive(Debug, Clone)]
struct SandboxPaths {
    root: PathBuf,
    home: PathBuf,
    workdir: PathBuf,
    xdg_config_home: PathBuf,
    xdg_cache_home: PathBuf,
    xdg_data_home: PathBuf,
    tmp: PathBuf,
    external_workdir: bool,
}

impl SandboxPaths {
    fn with_workdir(root: PathBuf, workdir: Option<PathBuf>) -> Self {
        let external_workdir = workdir.is_some();
        Self {
            home: root.join("home"),
            workdir: workdir.unwrap_or_else(|| root.join("cwd")),
            xdg_config_home: root.join("xdg-config"),
            xdg_cache_home: root.join("xdg-cache"),
            xdg_data_home: root.join("xdg-data"),
            tmp: root.join("tmp"),
            root,
            external_workdir,
        }
    }

    fn from_metadata(metadata: &SandboxMetadata) -> Self {
        Self {
            root: metadata.root.clone(),
            home: metadata.home.clone(),
            workdir: metadata.workdir.clone(),
            xdg_config_home: metadata.xdg_config_home.clone(),
            xdg_cache_home: metadata.xdg_cache_home.clone(),
            xdg_data_home: metadata.xdg_data_home.clone(),
            tmp: metadata.tmp.clone(),
            external_workdir: !metadata.workdir.starts_with(&metadata.root),
        }
    }
}

async fn create_execution_dirs(paths: &SandboxPaths) -> Result<()> {
    for path in [
        &paths.home,
        &paths.xdg_config_home,
        &paths.xdg_cache_home,
        &paths.xdg_data_home,
        &paths.tmp,
    ] {
        fs::create_dir_all(path)
            .await
            .map_err(|source| CliareError::CreateSandboxDir {
                path: path.to_path_buf(),
                source,
            })?;
    }
    if paths.external_workdir {
        ensure_existing_workdir(&paths.workdir).await?;
    } else {
        fs::create_dir_all(&paths.workdir).await.map_err(|source| {
            CliareError::CreateSandboxDir {
                path: paths.workdir.clone(),
                source,
            }
        })?;
    }
    Ok(())
}

async fn resolve_workdir(path: &Path) -> Result<PathBuf> {
    let metadata = fs::metadata(path)
        .await
        .map_err(|source| CliareError::ReadContextWorkdir {
            path: path.to_path_buf(),
            source,
        })?;
    if !metadata.is_dir() {
        return Err(CliareError::ContextWorkdirNotDirectory(path.to_path_buf()));
    }
    fs::canonicalize(path)
        .await
        .map_err(|source| CliareError::ReadContextWorkdir {
            path: path.to_path_buf(),
            source,
        })
}

async fn ensure_existing_workdir(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)
        .await
        .map_err(|source| CliareError::ReadContextWorkdir {
            path: path.to_path_buf(),
            source,
        })?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(CliareError::ContextWorkdirNotDirectory(path.to_path_buf()))
    }
}

fn process_sandbox(
    paths: SandboxPaths,
    env: BTreeMap<String, String>,
    snapshot_limits: SnapshotLimits,
) -> ProcessSandbox {
    ProcessSandbox {
        root: paths.root,
        cwd: paths.workdir.clone(),
        env,
        snapshot_limits,
        regions: vec![
            SandboxRegionRoot::new(SandboxRegion::Home, paths.home),
            SandboxRegionRoot::new_with_hash_mode(
                SandboxRegion::Workdir,
                paths.workdir,
                if paths.external_workdir {
                    SnapshotHashMode::Metadata
                } else {
                    SnapshotHashMode::Content
                },
            ),
            SandboxRegionRoot::new(SandboxRegion::XdgConfig, paths.xdg_config_home),
            SandboxRegionRoot::new(SandboxRegion::XdgCache, paths.xdg_cache_home),
            SandboxRegionRoot::new(SandboxRegion::XdgData, paths.xdg_data_home),
            SandboxRegionRoot::new(SandboxRegion::Tmp, paths.tmp),
        ],
    }
}

fn sandbox_env(paths: &SandboxPaths) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("CI".to_owned(), "1".to_owned());
    env.insert("CLIARE".to_owned(), "1".to_owned());
    env.insert("HOME".to_owned(), paths.home.display().to_string());
    env.insert("NO_COLOR".to_owned(), "1".to_owned());
    env.insert("PWD".to_owned(), paths.workdir.display().to_string());
    env.insert("TEMP".to_owned(), paths.tmp.display().to_string());
    env.insert("TERM".to_owned(), "dumb".to_owned());
    env.insert("TMP".to_owned(), paths.tmp.display().to_string());
    env.insert("TMPDIR".to_owned(), paths.tmp.display().to_string());
    env.insert(
        "XDG_CACHE_HOME".to_owned(),
        paths.xdg_cache_home.display().to_string(),
    );
    env.insert(
        "XDG_CONFIG_HOME".to_owned(),
        paths.xdg_config_home.display().to_string(),
    );
    env.insert(
        "XDG_DATA_HOME".to_owned(),
        paths.xdg_data_home.display().to_string(),
    );

    match std::env::var("PATH") {
        Ok(path) if !path.is_empty() => {
            env.insert("PATH".to_owned(), path);
        }
        _ => {
            env.insert(
                "PATH".to_owned(),
                "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".to_owned(),
            );
        }
    }
    if let Ok(lang) = std::env::var("LANG")
        && !lang.is_empty()
    {
        env.insert("LANG".to_owned(), lang);
    }
    if let Ok(locale) = std::env::var("LC_ALL")
        && !locale.is_empty()
    {
        env.insert("LC_ALL".to_owned(), locale);
    }

    env
}

fn host_env(cwd: &Path) -> BTreeMap<String, String> {
    let mut env = std::env::vars().collect::<BTreeMap<_, _>>();
    env.insert("PWD".to_owned(), cwd.display().to_string());
    env
}

fn process_host(
    cwd: PathBuf,
    env: BTreeMap<String, String>,
    snapshot_limits: SnapshotLimits,
) -> ProcessSandbox {
    ProcessSandbox {
        root: cwd.clone(),
        cwd,
        env,
        snapshot_limits,
        regions: Vec::new(),
    }
}

#[derive(Debug, Clone)]
struct SandboxRegionRoot {
    region: SandboxRegion,
    path: PathBuf,
    hash_mode: SnapshotHashMode,
}

impl SandboxRegionRoot {
    fn new(region: SandboxRegion, path: PathBuf) -> Self {
        Self::new_with_hash_mode(region, path, SnapshotHashMode::Content)
    }

    fn new_with_hash_mode(
        region: SandboxRegion,
        path: PathBuf,
        hash_mode: SnapshotHashMode,
    ) -> Self {
        Self {
            region,
            path,
            hash_mode,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotHashMode {
    Content,
    Metadata,
}

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

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{FileChangeKind, Sandbox, SandboxProfile, SandboxRegion, SnapshotLimits};

    #[tokio::test]
    async fn snapshots_report_created_modified_and_deleted_files_by_region() {
        let root = unique_test_dir("sandbox-side-effects");
        let sandbox = Sandbox::create(&root).await.expect("sandbox is created");
        let execution = sandbox.execution();

        let deleted_path = sandbox.metadata().home.join("deleted");
        let modified_path = sandbox.metadata().workdir.join("modified");
        tokio::fs::write(&deleted_path, "before")
            .await
            .expect("deleted fixture is written");
        tokio::fs::write(&modified_path, "before")
            .await
            .expect("modified fixture is written");

        let before = execution
            .snapshot()
            .await
            .expect("before snapshot succeeds");

        tokio::fs::remove_file(&deleted_path)
            .await
            .expect("deleted fixture is removed");
        tokio::fs::write(&modified_path, "after")
            .await
            .expect("modified fixture is changed");
        tokio::fs::write(sandbox.metadata().tmp.join("created"), "new")
            .await
            .expect("created fixture is written");

        let after = execution.snapshot().await.expect("after snapshot succeeds");
        let diff = before.diff(&after);

        assert_eq!(diff.created, 1);
        assert_eq!(diff.modified, 1);
        assert_eq!(diff.deleted, 1);
        assert!(diff.changes.iter().any(|change| {
            change.kind == FileChangeKind::Created && change.region == SandboxRegion::Tmp
        }));
        assert!(diff.changes.iter().any(|change| {
            change.kind == FileChangeKind::Modified && change.region == SandboxRegion::Workdir
        }));
        assert!(diff.changes.iter().any(|change| {
            change.kind == FileChangeKind::Deleted && change.region == SandboxRegion::Home
        }));

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn provided_workdir_uses_metadata_snapshots() {
        let root = unique_test_dir("sandbox-provided-workdir");
        let out_dir = root.join("out");
        let workdir = root.join("project");
        tokio::fs::create_dir_all(&workdir)
            .await
            .expect("provided workdir is created");
        let tracked = workdir.join("tracked.txt");
        tokio::fs::write(&tracked, "before")
            .await
            .expect("tracked fixture is written");

        let sandbox = Sandbox::create_with_workdir(&out_dir, Some(&workdir))
            .await
            .expect("sandbox is created with provided workdir");
        let execution = sandbox.execution();
        let before = execution
            .snapshot()
            .await
            .expect("before snapshot succeeds");

        tokio::fs::write(&tracked, "after with different length")
            .await
            .expect("tracked fixture is modified");

        let after = execution.snapshot().await.expect("after snapshot succeeds");
        let diff = before.diff(&after);

        assert_eq!(diff.modified, 1);
        let change = diff
            .changes
            .iter()
            .find(|change| change.region == SandboxRegion::Workdir)
            .expect("workdir change is reported");
        assert_eq!(change.kind, FileChangeKind::Modified);
        assert_eq!(change.sha256, None);

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn snapshot_reports_truncation_when_file_budget_is_exhausted() {
        let root = unique_test_dir("sandbox-snapshot-budget");
        let sandbox = Sandbox::create(&root).await.expect("sandbox is created");
        let execution = sandbox.execution();

        let before = execution
            .snapshot()
            .await
            .expect("before snapshot succeeds");
        for index in 0..40 {
            tokio::fs::write(
                sandbox.metadata().tmp.join(format!("created-{index}")),
                "new",
            )
            .await
            .expect("created fixture is written");
        }
        let after = execution.snapshot().await.expect("after snapshot succeeds");
        let diff = before.diff(&after);

        assert!(diff.truncated);
        assert_eq!(
            diff.truncation_reason.as_deref(),
            Some("file_budget_exhausted")
        );

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn snapshot_uses_limits_supplied_by_profile_configuration() {
        let root = unique_test_dir("sandbox-configured-snapshot-budget");
        let limits = SnapshotLimits::new(1, 64, 1024);
        let sandbox =
            Sandbox::create_with_profile_and_limits(&root, None, SandboxProfile::Isolated, limits)
                .await
                .expect("sandbox is created");
        let execution = sandbox
            .execution_for_probe("p_000001")
            .await
            .expect("probe execution is created");

        assert_eq!(sandbox.metadata().snapshot_limits, limits);
        assert!(!sandbox.metadata().hostile_binary_containment);
        let evidence = sandbox.probe_evidence_for(&execution);
        assert_eq!(evidence.snapshot_limits, limits);
        assert!(!evidence.hostile_binary_containment);

        let before = execution
            .snapshot()
            .await
            .expect("before snapshot succeeds");
        tokio::fs::write(execution.cwd.join("one"), "new")
            .await
            .expect("first fixture is written");
        tokio::fs::write(execution.cwd.join("two"), "new")
            .await
            .expect("second fixture is written");
        let after = execution.snapshot().await.expect("after snapshot succeeds");
        let diff = before.diff(&after);

        assert!(diff.truncated);
        assert_eq!(
            diff.truncation_reason.as_deref(),
            Some("file_budget_exhausted")
        );

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cliare-{name}-{}-{nonce}", std::process::id()))
    }
}
