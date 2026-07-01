use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use tokio::fs;

use cliare_core::artifacts::{CONTEXT_SUITE_JSON, RUNTIME_CONTEXT_JSON, SCORECARD_JSON};
use cliare_core::error::{CliareError, Result};

use super::runtime::{PersistedContext, RuntimeContext, sanitize_context_name};

pub fn measurement_dir(root: &Path, context: &RuntimeContext) -> PathBuf {
    if context.is_context_suite_measurement() {
        contexts_dir(root).join(context.folder_name())
    } else {
        root.to_path_buf()
    }
}

pub fn context_artifact_dir(root: &Path, context: &str) -> PathBuf {
    contexts_dir(root).join(sanitize_context_name(context))
}

pub fn contexts_dir(root: &Path) -> PathBuf {
    root.join("contexts")
}

pub async fn persisted_contexts(root: &Path) -> Result<Vec<PersistedContext>> {
    let dirs = discover_context_dirs(root).await?;
    let mut contexts = Vec::new();
    for dir in dirs {
        let runtime_context = read_runtime_context(&dir).await?;
        let fallback_name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("context")
            .to_owned();
        contexts.push(PersistedContext {
            name: runtime_context
                .as_ref()
                .map(|context| context.name.clone())
                .unwrap_or(fallback_name),
            profile: runtime_context.map(|context| context.profile),
            artifact_dir: dir,
        });
    }
    contexts.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(contexts)
}

pub async fn resolve_measurement_dir(
    root: &Path,
    context: Option<&str>,
    command: &str,
) -> Result<PathBuf> {
    if let Some(context) = context {
        let artifact_dir = context_artifact_dir(root, context);
        if artifact_dir.join(SCORECARD_JSON).is_file() {
            return Ok(artifact_dir);
        }
        let contexts = persisted_contexts(root).await?;
        return Err(CliareError::ContextSelectionNotFound {
            message: context_not_found_message(root, command, context, &contexts),
        });
    }

    if root.join(SCORECARD_JSON).is_file() {
        return Ok(root.to_path_buf());
    }

    let contexts = persisted_contexts(root).await?;
    match contexts.as_slice() {
        [] => Err(invalid_measurement_artifact(root, command).await?),
        [context] => Ok(context.artifact_dir.clone()),
        _ => Err(CliareError::ContextSelectionRequired {
            message: context_required_message(root, command, &contexts),
        }),
    }
}

pub async fn is_context_suite_root(root: &Path) -> Result<bool> {
    if root.join(CONTEXT_SUITE_JSON).is_file() {
        return Ok(true);
    }
    Ok(!persisted_contexts(root).await?.is_empty())
}

pub async fn write_runtime_context(out_dir: &Path, context: &RuntimeContext) -> Result<PathBuf> {
    fs::create_dir_all(out_dir)
        .await
        .map_err(|source| CliareError::CreateArtifactDir {
            path: out_dir.to_path_buf(),
            source,
        })?;
    let path = out_dir.join(RUNTIME_CONTEXT_JSON);
    let bytes = serde_json::to_vec_pretty(context).map_err(CliareError::SerializeRuntimeContext)?;
    fs::write(&path, bytes)
        .await
        .map_err(|source| CliareError::WriteRuntimeContext {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}

pub(super) async fn discover_context_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let dir = contexts_dir(root);
    let mut entries = match fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(CliareError::ReadContextSuiteDir { path: dir, source });
        }
    };
    let mut dirs = Vec::new();
    while let Some(entry) =
        entries
            .next_entry()
            .await
            .map_err(|source| CliareError::ReadContextSuiteDir {
                path: dir.clone(),
                source,
            })?
    {
        let path = entry.path();
        let metadata =
            entry
                .metadata()
                .await
                .map_err(|source| CliareError::ReadContextSuiteDir {
                    path: path.clone(),
                    source,
                })?;
        if metadata.is_dir() && path.join(SCORECARD_JSON).is_file() {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
}

pub(super) async fn read_runtime_context(dir: &Path) -> Result<Option<RuntimeContext>> {
    let path = dir.join(RUNTIME_CONTEXT_JSON);
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliareError::ReadContextScorecard { path, source });
        }
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| CliareError::ParseContextScorecard { path, source })
}

pub(super) fn context_required_message(
    root: &Path,
    command: &str,
    contexts: &[PersistedContext],
) -> String {
    format!(
        "{command} needs a concrete measurement context: {} is a context suite with persisted contexts: {}. Use --context <name> or pass a context artifact directory such as {}.",
        root.display(),
        context_list(contexts),
        contexts
            .first()
            .map(|context| context.artifact_dir.display().to_string())
            .unwrap_or_else(|| root.display().to_string())
    )
}

pub(super) fn context_not_found_message(
    root: &Path,
    command: &str,
    requested: &str,
    contexts: &[PersistedContext],
) -> String {
    format!(
        "{command} could not find context `{}` under {}. Persisted contexts: {}.",
        requested,
        root.display(),
        if contexts.is_empty() {
            "none".to_owned()
        } else {
            context_list(contexts)
        }
    )
}

pub(super) fn context_list(contexts: &[PersistedContext]) -> String {
    contexts
        .iter()
        .map(|context| format!("{} ({})", context.name, context.artifact_dir.display()))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) async fn invalid_measurement_artifact(
    root: &Path,
    command: &str,
) -> Result<CliareError> {
    let metadata = match fs::metadata(root).await {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == ErrorKind::NotFound => {
            return Ok(CliareError::MeasurementArtifactNotFound {
                command: command.to_owned(),
                path: root.to_path_buf(),
            });
        }
        Err(source) => {
            return Err(CliareError::ReadArtifactDirectory {
                path: root.to_path_buf(),
                source,
            });
        }
    };

    if !metadata.is_dir() {
        return Ok(CliareError::ArtifactPathNotDirectory {
            path: root.to_path_buf(),
        });
    }

    let child_dirs = child_directory_names(root).await?;
    let hint = if child_dirs.is_empty() {
        format!(
            "Run `cliare measure <target-cli> --out {}` first, or pass --out to an existing measurement directory.",
            root.display()
        )
    } else {
        format!(
            "This looks like a workspace root with child directories: {}. Pass --out {}/<project>, add --context <name> when that project has multiple contexts, or pass a concrete context directory.",
            child_dirs.join(", "),
            root.display()
        )
    };

    Ok(CliareError::InvalidMeasurementArtifact {
        message: format!(
            "{command} needs a CLIARE measurement artifact directory, but {} does not contain scorecard.json or contexts/<context>/scorecard.json. {hint}",
            root.display()
        ),
    })
}

pub(super) async fn child_directory_names(root: &Path) -> Result<Vec<String>> {
    let mut entries =
        fs::read_dir(root)
            .await
            .map_err(|source| CliareError::ReadArtifactDirectory {
                path: root.to_path_buf(),
                source,
            })?;
    let mut dirs = Vec::new();
    while let Some(entry) =
        entries
            .next_entry()
            .await
            .map_err(|source| CliareError::ReadArtifactDirectory {
                path: root.to_path_buf(),
                source,
            })?
    {
        let metadata =
            entry
                .metadata()
                .await
                .map_err(|source| CliareError::ReadArtifactDirectory {
                    path: entry.path(),
                    source,
                })?;
        if metadata.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            dirs.push(name.to_owned());
        }
    }
    dirs.sort();
    Ok(dirs)
}
