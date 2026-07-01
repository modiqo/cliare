use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use super::RUNTIME_CONTEXT_SCHEMA_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextProfile {
    Single,
    Clean,
    Authenticated,
    LocalContext,
    Fixture,
    Custom,
}

impl RuntimeContextProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Clean => "clean",
            Self::Authenticated => "authenticated",
            Self::LocalContext => "local_context",
            Self::Fixture => "fixture",
            Self::Custom => "custom",
        }
    }

    pub fn folder_name(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Clean => "clean",
            Self::Authenticated => "authenticated",
            Self::LocalContext => "local-context",
            Self::Fixture => "fixture",
            Self::Custom => "custom",
        }
    }

    pub fn cli_value(self) -> &'static str {
        self.folder_name()
    }

    pub fn default_auth_state(self) -> RuntimeContextState {
        match self {
            Self::Clean => RuntimeContextState::Absent,
            Self::Authenticated => RuntimeContextState::Present,
            Self::Single | Self::LocalContext | Self::Fixture | Self::Custom => {
                RuntimeContextState::Unknown
            }
        }
    }

    pub fn default_local_context_state(self) -> RuntimeContextState {
        match self {
            Self::Clean | Self::Authenticated => RuntimeContextState::Absent,
            Self::LocalContext => RuntimeContextState::Present,
            Self::Single | Self::Fixture | Self::Custom => RuntimeContextState::Unknown,
        }
    }

    pub fn default_fixture_state(self) -> RuntimeContextState {
        match self {
            Self::Clean | Self::Authenticated | Self::LocalContext => RuntimeContextState::Absent,
            Self::Fixture => RuntimeContextState::Present,
            Self::Single | Self::Custom => RuntimeContextState::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextState {
    Absent,
    Present,
    Unknown,
    Declared,
}

impl RuntimeContextState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Present => "present",
            Self::Unknown => "unknown",
            Self::Declared => "declared",
        }
    }

    pub fn cli_value(self) -> &'static str {
        self.label()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextCwdPolicy {
    Isolated,
    Provided,
}

impl RuntimeContextCwdPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
            Self::Provided => "provided",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RuntimeContext {
    pub schema_version: String,
    pub profile: RuntimeContextProfile,
    pub name: String,
    pub auth_state: RuntimeContextState,
    pub local_context_state: RuntimeContextState,
    pub fixture_state: RuntimeContextState,
    pub network_state: RuntimeContextState,
    pub runtime_dependency_state: RuntimeContextState,
    pub cwd_policy: RuntimeContextCwdPolicy,
    pub workdir: Option<PathBuf>,
    pub declared_by: RuntimeContextDeclaration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextDeclaration {
    Default,
    Cli,
}

#[derive(Debug, Clone)]
pub struct RuntimeContextInput {
    pub profile: Option<RuntimeContextProfile>,
    pub name: Option<String>,
    pub auth_state: Option<RuntimeContextState>,
    pub local_context_state: Option<RuntimeContextState>,
    pub fixture_state: Option<RuntimeContextState>,
    pub network_state: Option<RuntimeContextState>,
    pub runtime_dependency_state: Option<RuntimeContextState>,
    pub workdir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PersistedContext {
    pub name: String,
    pub profile: Option<RuntimeContextProfile>,
    pub artifact_dir: PathBuf,
}

impl RuntimeContext {
    pub fn from_input(input: RuntimeContextInput) -> Self {
        let profile = input.profile.unwrap_or(RuntimeContextProfile::Single);
        let declared_by = if input.profile.is_some()
            || input.name.is_some()
            || input.auth_state.is_some()
            || input.local_context_state.is_some()
            || input.fixture_state.is_some()
            || input.network_state.is_some()
            || input.runtime_dependency_state.is_some()
            || input.workdir.is_some()
        {
            RuntimeContextDeclaration::Cli
        } else {
            RuntimeContextDeclaration::Default
        };
        let cwd_policy = if input.workdir.is_some() {
            RuntimeContextCwdPolicy::Provided
        } else {
            RuntimeContextCwdPolicy::Isolated
        };
        let local_context_state = input.local_context_state.unwrap_or_else(|| {
            if input.workdir.is_some() {
                RuntimeContextState::Present
            } else {
                profile.default_local_context_state()
            }
        });

        Self {
            schema_version: RUNTIME_CONTEXT_SCHEMA_VERSION.to_owned(),
            profile,
            name: input
                .name
                .unwrap_or_else(|| profile.folder_name().to_owned()),
            auth_state: input
                .auth_state
                .unwrap_or_else(|| profile.default_auth_state()),
            local_context_state,
            fixture_state: input
                .fixture_state
                .unwrap_or_else(|| profile.default_fixture_state()),
            network_state: input.network_state.unwrap_or(RuntimeContextState::Unknown),
            runtime_dependency_state: input
                .runtime_dependency_state
                .unwrap_or(RuntimeContextState::Unknown),
            cwd_policy,
            workdir: input.workdir,
            declared_by,
        }
    }

    pub fn is_context_suite_measurement(&self) -> bool {
        self.profile != RuntimeContextProfile::Single
    }

    pub fn folder_name(&self) -> String {
        sanitize_context_name(&self.name)
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::from_input(RuntimeContextInput {
            profile: None,
            name: None,
            auth_state: None,
            local_context_state: None,
            fixture_state: None,
            network_state: None,
            runtime_dependency_state: None,
            workdir: None,
        })
    }
}

pub(super) fn sanitize_context_name(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.' | ' ') && !sanitized.ends_with('-') {
            sanitized.push('-');
        }
    }
    let sanitized = sanitized.trim_matches('-').to_owned();
    if sanitized.is_empty() {
        "context".to_owned()
    } else {
        sanitized
    }
}
