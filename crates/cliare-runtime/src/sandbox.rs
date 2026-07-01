use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[cfg(not(test))]
pub const DEFAULT_SNAPSHOT_MAX_FILES: usize = 10_000;
#[cfg(test)]
pub const DEFAULT_SNAPSHOT_MAX_FILES: usize = 32;
#[cfg(not(test))]
pub const DEFAULT_SNAPSHOT_MAX_DIRS: usize = 2_000;
#[cfg(test)]
pub const DEFAULT_SNAPSHOT_MAX_DIRS: usize = 32;
pub const DEFAULT_SNAPSHOT_MAX_HASH_BYTES: u64 = 64 * 1024 * 1024;

use cliare_core::error::{CliareError, Result};

mod snapshot;
#[cfg(test)]
mod tests;

pub use snapshot::{
    FileChange, FileChangeKind, SandboxRegion, SideEffectSnapshot, SideEffectSummary,
};

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
