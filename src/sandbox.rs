use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

use crate::error::{CliareError, Result};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
}

#[derive(Debug, Clone)]
pub struct ProcessSandbox {
    pub root: PathBuf,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    regions: Vec<SandboxRegionRoot>,
}

#[derive(Debug, Clone)]
pub struct Sandbox {
    metadata: SandboxMetadata,
    env: BTreeMap<String, String>,
    provided_workdir: Option<PathBuf>,
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
        match profile {
            SandboxProfile::Isolated => Self::create_isolated(out_dir, workdir).await,
            SandboxProfile::Host => Self::create_host(out_dir, workdir).await,
        }
    }

    async fn create_isolated(out_dir: &Path, workdir: Option<&Path>) -> Result<Self> {
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
        };

        Ok(Self {
            metadata,
            env,
            provided_workdir,
        })
    }

    async fn create_host(out_dir: &Path, workdir: Option<&Path>) -> Result<Self> {
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
        };

        Ok(Self {
            metadata,
            env,
            provided_workdir,
        })
    }

    pub fn metadata(&self) -> &SandboxMetadata {
        &self.metadata
    }

    pub fn execution(&self) -> ProcessSandbox {
        process_sandbox(
            SandboxPaths::from_metadata(&self.metadata),
            self.env.clone(),
        )
    }

    pub async fn execution_for_probe(&self, probe_id: &str) -> Result<ProcessSandbox> {
        if self.metadata.profile == SandboxProfile::Host {
            return Ok(process_host(
                self.metadata.workdir.clone(),
                self.env.clone(),
            ));
        }
        let paths = SandboxPaths::with_workdir(
            self.metadata.root.join("probes").join(probe_id),
            self.provided_workdir.clone(),
        );
        create_execution_dirs(&paths).await?;
        Ok(process_sandbox(paths.clone(), sandbox_env(&paths)))
    }

    pub fn probe_evidence(&self) -> ProbeSandboxEvidence {
        ProbeSandboxEvidence {
            profile: self.metadata.profile,
            cwd: self.metadata.workdir.clone(),
            env_policy: self.metadata.env_policy,
            env_keys: self.metadata.env_keys.clone(),
        }
    }

    pub fn probe_evidence_for(&self, execution: &ProcessSandbox) -> ProbeSandboxEvidence {
        ProbeSandboxEvidence {
            profile: self.metadata.profile,
            cwd: execution.cwd.clone(),
            env_policy: self.metadata.env_policy,
            env_keys: self.metadata.env_keys.clone(),
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

fn process_sandbox(paths: SandboxPaths, env: BTreeMap<String, String>) -> ProcessSandbox {
    ProcessSandbox {
        root: paths.root,
        cwd: paths.workdir.clone(),
        env,
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

fn process_host(cwd: PathBuf, env: BTreeMap<String, String>) -> ProcessSandbox {
    ProcessSandbox {
        root: cwd.clone(),
        cwd,
        env,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxRegion {
    Home,
    Workdir,
    XdgConfig,
    XdgCache,
    XdgData,
    Tmp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileChange {
    pub kind: FileChangeKind,
    pub region: SandboxRegion,
    pub path: PathBuf,
    pub size_bytes: Option<u64>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SideEffectSummary {
    pub created: usize,
    pub modified: usize,
    pub deleted: usize,
    pub total: usize,
    pub changes: Vec<FileChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideEffectSnapshot {
    files: BTreeMap<PathBuf, FileSnapshot>,
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

        for region_root in &self.regions {
            scan_region(&self.root, region_root, &mut files).await?;
        }

        Ok(SideEffectSnapshot { files })
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
            changes,
        }
    }
}

async fn scan_region(
    sandbox_root: &Path,
    region_root: &SandboxRegionRoot,
    files: &mut BTreeMap<PathBuf, FileSnapshot>,
) -> Result<()> {
    let mut pending = vec![region_root.path.clone()];

    while let Some(dir) = pending.pop() {
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
                let sha256 = match region_root.hash_mode {
                    SnapshotHashMode::Content => sha256_file(&path).await?,
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

async fn sha256_file(path: &Path) -> Result<Option<String>> {
    let mut file = match File::open(path).await {
        Ok(file) => file,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliareError::ReadSandboxFile {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

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
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(Some(format!("{:x}", hasher.finalize())))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{FileChangeKind, Sandbox, SandboxRegion};

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

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cliare-{name}-{}-{nonce}", std::process::id()))
    }
}
