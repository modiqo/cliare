use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{CliareError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxProfile {
    Isolated,
}

impl SandboxProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
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
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Sandbox {
    metadata: SandboxMetadata,
    env: BTreeMap<String, String>,
}

impl Sandbox {
    pub async fn create(out_dir: &Path) -> Result<Self> {
        let root = out_dir.join("sandbox");
        let home = root.join("home");
        let workdir = root.join("cwd");
        let xdg_config_home = root.join("xdg-config");
        let xdg_cache_home = root.join("xdg-cache");
        let xdg_data_home = root.join("xdg-data");
        let tmp = root.join("tmp");

        for path in [
            &home,
            &workdir,
            &xdg_config_home,
            &xdg_cache_home,
            &xdg_data_home,
            &tmp,
        ] {
            fs::create_dir_all(path)
                .await
                .map_err(|source| CliareError::CreateSandboxDir {
                    path: path.to_path_buf(),
                    source,
                })?;
        }

        let mut env = BTreeMap::new();
        env.insert("CI".to_owned(), "1".to_owned());
        env.insert("CLIARE".to_owned(), "1".to_owned());
        env.insert("HOME".to_owned(), home.display().to_string());
        env.insert("NO_COLOR".to_owned(), "1".to_owned());
        env.insert("PWD".to_owned(), workdir.display().to_string());
        env.insert("TEMP".to_owned(), tmp.display().to_string());
        env.insert("TERM".to_owned(), "dumb".to_owned());
        env.insert("TMP".to_owned(), tmp.display().to_string());
        env.insert("TMPDIR".to_owned(), tmp.display().to_string());
        env.insert(
            "XDG_CACHE_HOME".to_owned(),
            xdg_cache_home.display().to_string(),
        );
        env.insert(
            "XDG_CONFIG_HOME".to_owned(),
            xdg_config_home.display().to_string(),
        );
        env.insert(
            "XDG_DATA_HOME".to_owned(),
            xdg_data_home.display().to_string(),
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

        let env_keys = env.keys().cloned().collect();
        let metadata = SandboxMetadata {
            profile: SandboxProfile::Isolated,
            root,
            home,
            workdir,
            xdg_config_home,
            xdg_cache_home,
            xdg_data_home,
            tmp,
            env_policy: EnvPolicy::ClearedWithAllowlist,
            env_keys,
        };

        Ok(Self { metadata, env })
    }

    pub fn metadata(&self) -> &SandboxMetadata {
        &self.metadata
    }

    pub fn execution(&self) -> ProcessSandbox {
        ProcessSandbox {
            cwd: self.metadata.workdir.clone(),
            env: self.env.clone(),
        }
    }

    pub fn probe_evidence(&self) -> ProbeSandboxEvidence {
        ProbeSandboxEvidence {
            profile: self.metadata.profile,
            cwd: self.metadata.workdir.clone(),
            env_policy: self.metadata.env_policy,
            env_keys: self.metadata.env_keys.clone(),
        }
    }
}
