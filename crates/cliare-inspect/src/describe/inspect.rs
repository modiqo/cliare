use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::Value;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

use cliare_core::error::{CliareError, Result};

use super::model::{ArtifactFile, FileKind, FileSpec};

pub(super) async fn read_top_level(folder: &Path) -> Result<BTreeSet<String>> {
    let mut entries = BTreeSet::new();
    let mut dir =
        fs::read_dir(folder)
            .await
            .map_err(|source| CliareError::ReadArtifactDirectory {
                path: folder.to_path_buf(),
                source,
            })?;
    while let Some(entry) =
        dir.next_entry()
            .await
            .map_err(|source| CliareError::ReadArtifactDirectory {
                path: folder.to_path_buf(),
                source,
            })?
    {
        if let Some(name) = entry.file_name().to_str() {
            entries.insert(name.to_owned());
        }
    }
    Ok(entries)
}

pub(super) async fn dynamic_artifact_paths(
    folder: &Path,
    top_level: &BTreeSet<String>,
) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    for name in top_level {
        if is_persona_file(name)
            || matches!(
                name.as_str(),
                "artifact-map.json" | "artifact-map.md" | "sandbox"
            )
        {
            paths.push(name.clone());
        }
    }

    let jobs_dir = folder.join("jobs");
    if let Ok(mut entries) = fs::read_dir(&jobs_dir).await {
        while let Some(entry) =
            entries
                .next_entry()
                .await
                .map_err(|source| CliareError::ReadArtifactDirectory {
                    path: jobs_dir.clone(),
                    source,
                })?
        {
            if let Some(name) = entry.file_name().to_str() {
                paths.push(format!("jobs/{name}"));
            }
        }
    }

    for name in top_level {
        let path = folder.join(name);
        if fs::metadata(&path)
            .await
            .is_ok_and(|metadata| metadata.is_dir())
            && name != "jobs"
            && name != "sandbox"
            && path.join("scorecard.json").is_file()
        {
            paths.push(name.clone());
        }
    }

    Ok(paths)
}

fn is_persona_file(name: &str) -> bool {
    name.starts_with("persona-") && (name.ends_with(".json") || name.ends_with(".md"))
}

pub(super) fn dynamic_file_spec(path: String) -> FileSpec {
    if path == "artifact-map.json" {
        FileSpec::new(
            path,
            FileKind::ArtifactMap,
            "Machine-readable map of the artifact directory.",
            false,
            0,
            "Use as the first file an agent reads when present.",
        )
    } else if path == "artifact-map.md" {
        FileSpec::new(
            path,
            FileKind::ArtifactMapReport,
            "Human-readable map of the artifact directory.",
            false,
            0,
            "Use as the first human-facing orientation file when present.",
        )
    } else if path.starts_with("persona-") && path.ends_with(".json") {
        FileSpec::new(
            path,
            FileKind::PersonaOutcome,
            "Persona-specific JSON outcome packet.",
            false,
            12,
            "Use after selecting the reviewer persona.",
        )
    } else if path.starts_with("persona-") && path.ends_with(".md") {
        FileSpec::new(
            path,
            FileKind::PersonaReport,
            "Persona-specific Markdown report.",
            false,
            11,
            "Use for the role-specific action brief.",
        )
    } else if path.starts_with("jobs/") && path.ends_with(".stdout.log") {
        FileSpec::new(
            path,
            FileKind::JobStdout,
            "Captured stdout from a detached measurement worker.",
            false,
            33,
            "Inspect when the worker summary or command output is needed.",
        )
    } else if path.starts_with("jobs/") && path.ends_with(".stderr.log") {
        FileSpec::new(
            path,
            FileKind::JobStderr,
            "Captured stderr from a detached measurement worker.",
            false,
            34,
            "Inspect when a detached worker fails or emits diagnostics.",
        )
    } else if path.starts_with("jobs/") && path.ends_with(".log") {
        FileSpec::new(
            path,
            FileKind::JobLog,
            "Progress log for a foreground or detached measurement job.",
            false,
            32,
            "Use for live progress and artifact-writing milestones.",
        )
    } else if path == "sandbox" {
        FileSpec::new(
            path,
            FileKind::Sandbox,
            "Per-probe sandbox filesystem evidence.",
            false,
            80,
            "Inspect only when investigating side effects or probe isolation.",
        )
    } else {
        FileSpec::new(
            path,
            FileKind::Additional,
            "Discovered artifact that is not part of the minimum CLIARE contract.",
            false,
            90,
            "Inspect only after the canonical artifacts are understood.",
        )
    }
}

pub(super) async fn inspect_entry(folder: &Path, spec: FileSpec) -> ArtifactFile {
    let path = folder.join(&spec.path);
    let metadata = fs::metadata(&path).await;
    let mut entry = ArtifactFile {
        path: spec.path,
        kind: spec.kind,
        role: spec.role,
        required: spec.required,
        exists: false,
        size_bytes: None,
        records: None,
        schema_version: None,
        parse_status: None,
        navigation_rank: spec.navigation_rank,
        agent_use: spec.agent_use,
    };

    match metadata {
        Ok(metadata) => {
            entry.exists = true;
            entry.size_bytes = metadata.is_file().then_some(metadata.len());
            if metadata.is_dir() {
                entry.kind = match entry.path.as_str() {
                    "sandbox" => FileKind::Sandbox,
                    "contexts" => FileKind::ContextsDirectory,
                    _ => FileKind::Directory,
                };
                return entry;
            }
        }
        Err(source) if source.kind() == ErrorKind::NotFound => return entry,
        Err(source) => {
            entry.parse_status = Some(format!("metadata_error: {source}"));
            return entry;
        }
    }

    if entry.path.ends_with(".json") || entry.path.ends_with(".sarif") {
        match read_json_value(&path).await {
            Ok(value) => {
                entry.schema_version = schema_version(&value);
                entry.parse_status = Some("ok".to_owned());
            }
            Err(error) => {
                entry.parse_status = Some(format!("parse_error: {error}"));
            }
        }
    } else if entry.path.ends_with(".jsonl") {
        match count_jsonl_records(&path).await {
            Ok(records) => {
                entry.records = Some(records);
                entry.parse_status = Some("ok".to_owned());
            }
            Err(error) => {
                entry.parse_status = Some(format!("read_error: {error}"));
            }
        }
    }

    entry
}

pub(super) async fn read_json_value(path: &Path) -> std::result::Result<Value, String> {
    let text = fs::read_to_string(path)
        .await
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

async fn count_jsonl_records(path: &Path) -> std::result::Result<u64, String> {
    let file = fs::File::open(path)
        .await
        .map_err(|error| error.to_string())?;
    let mut lines = BufReader::new(file).lines();
    let mut records = 0_u64;
    while let Some(line) = lines.next_line().await.map_err(|error| error.to_string())? {
        if !line.trim().is_empty() {
            records += 1;
        }
    }
    Ok(records)
}

fn schema_version(value: &Value) -> Option<String> {
    value
        .get("schema_version")
        .and_then(Value::as_str)
        .map(str::to_owned)
}
