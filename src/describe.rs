use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::cli::{DescribeArgs, DescribeFormat, JobsArgs, JobsCommand, JobsStatusArgs};
use crate::context;
use crate::error::{CliareError, Result};

const ARTIFACT_MAP_SCHEMA_VERSION: &str = "cliare.artifact-map.v1";

#[derive(Debug)]
pub struct DescribeSummary {
    pub folder: PathBuf,
    pub artifact_kind: ArtifactKind,
    pub files_total: usize,
    pub missing_required: usize,
    pub artifact_map_json_path: Option<PathBuf>,
    pub artifact_map_markdown_path: Option<PathBuf>,
    rendered: String,
}

impl DescribeSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.rendered
    }
}

pub async fn describe(args: DescribeArgs) -> Result<DescribeSummary> {
    let folder = if args.context.is_some() {
        context::resolve_measurement_dir(&args.folder, args.context.as_deref(), "cliare describe")
            .await?
    } else {
        args.folder.clone()
    };
    ensure_directory(&folder).await?;
    let mut map = build_artifact_map(&folder).await?;
    let mut artifact_map_json_path = None;
    let mut artifact_map_markdown_path = None;

    if args.write {
        let json_path = folder.join("artifact-map.json");
        let markdown_path = folder.join("artifact-map.md");
        map.written_files = vec![
            relative_to(&folder, &json_path),
            relative_to(&folder, &markdown_path),
        ];
        write_json(&json_path, &map).await?;
        write_markdown(&markdown_path, &render_markdown(&map)).await?;
        map = build_artifact_map(&folder).await?;
        map.written_files = vec![
            relative_to(&folder, &json_path),
            relative_to(&folder, &markdown_path),
        ];
        write_json(&json_path, &map).await?;
        write_markdown(&markdown_path, &render_markdown(&map)).await?;
        artifact_map_json_path = Some(json_path);
        artifact_map_markdown_path = Some(markdown_path);
    }

    let rendered = match args.format {
        DescribeFormat::Markdown => render_markdown(&map),
        DescribeFormat::Json => {
            serde_json::to_string_pretty(&map).map_err(CliareError::SerializeArtifactMap)? + "\n"
        }
    };

    Ok(DescribeSummary {
        folder,
        artifact_kind: map.artifact_kind,
        files_total: map.files.len(),
        missing_required: map.missing_required.len(),
        artifact_map_json_path,
        artifact_map_markdown_path,
        rendered,
    })
}

async fn ensure_directory(path: &Path) -> Result<()> {
    let metadata =
        fs::metadata(path)
            .await
            .map_err(|source| CliareError::ReadArtifactDirectory {
                path: path.to_path_buf(),
                source,
            })?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(CliareError::ArtifactPathNotDirectory {
            path: path.to_path_buf(),
        })
    }
}

async fn build_artifact_map(folder: &Path) -> Result<ArtifactMap> {
    let top_level = read_top_level(folder).await?;
    let artifact_kind = detect_artifact_kind(&top_level);
    let mut known = BTreeSet::new();
    let mut files = Vec::new();

    for spec in known_file_specs(artifact_kind) {
        known.insert(spec.path.to_owned());
        files.push(inspect_entry(folder, spec).await);
    }

    for path in dynamic_artifact_paths(folder, &top_level).await? {
        if known.insert(path.clone()) {
            files.push(inspect_entry(folder, dynamic_file_spec(path)).await);
        }
    }

    files.sort_by(|left, right| {
        left.navigation_rank
            .cmp(&right.navigation_rank)
            .then_with(|| left.path.cmp(&right.path))
    });

    let missing_required = files
        .iter()
        .filter(|file| file.required && !file.exists)
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();

    let summaries = ArtifactSummaries {
        scorecard: scorecard_summary(folder).await,
        command_index: command_index_summary(folder).await,
        issues: issues_summary(folder).await,
        benchmark: benchmark_summary(folder).await,
        contexts: contexts_summary(folder).await,
        job: job_summary(folder).await,
    };

    Ok(ArtifactMap {
        schema_version: ARTIFACT_MAP_SCHEMA_VERSION.to_owned(),
        cliare_version: env!("CARGO_PKG_VERSION").to_owned(),
        generated_at: timestamp()?,
        folder: folder.display().to_string(),
        artifact_kind,
        health: health(&missing_required, &summaries),
        navigation: navigation_plan(artifact_kind),
        summaries,
        missing_required,
        files,
        written_files: Vec::new(),
    })
}

async fn read_top_level(folder: &Path) -> Result<BTreeSet<String>> {
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

fn detect_artifact_kind(top_level: &BTreeSet<String>) -> ArtifactKind {
    let measurement = top_level.contains("scorecard.json") || top_level.contains("evidence.jsonl");
    let benchmark = top_level.contains("benchmark.json");
    let context_suite = top_level.contains("context-suite.json")
        || top_level.contains("context-compare.md")
        || top_level.contains("contexts");
    match (measurement, benchmark, context_suite) {
        (false, false, true) => ArtifactKind::ContextSuite,
        (true, true, _) => ArtifactKind::Mixed,
        (true, false, _) => ArtifactKind::Measurement,
        (false, true, _) => ArtifactKind::Benchmark,
        (false, false, false) => ArtifactKind::Unknown,
    }
}

fn known_file_specs(kind: ArtifactKind) -> Vec<FileSpec> {
    match kind {
        ArtifactKind::Measurement | ArtifactKind::Mixed => measurement_specs(),
        ArtifactKind::Benchmark => benchmark_specs(),
        ArtifactKind::ContextSuite => context_suite_specs(),
        ArtifactKind::Unknown => common_specs(),
    }
}

fn measurement_specs() -> Vec<FileSpec> {
    vec![
        FileSpec::new(
            "scorecard.json",
            FileKind::Scorecard,
            "Compact score, subscores, coverage pressure, findings, and provenance.",
            true,
            1,
            "Start here to understand posture and whether the run is complete.",
        ),
        FileSpec::new(
            "summary.md",
            FileKind::CiSummary,
            "Markdown summary intended for CI job summaries and pull request checks.",
            false,
            2,
            "Use for a concise human-readable overview.",
        ),
        FileSpec::new(
            "issues.json",
            FileKind::IssueLedger,
            "Canonical issue ledger with affected commands, evidence, recommendations, and verification commands.",
            true,
            3,
            "Use as the remediation work queue.",
        ),
        FileSpec::new(
            "issues.md",
            FileKind::IssueReport,
            "Markdown rendering of the issue ledger.",
            false,
            4,
            "Use when a human needs the same issue queue in report form.",
        ),
        FileSpec::new(
            "command-index.json",
            FileKind::CommandIndex,
            "Command-centric lookup table with parameters, preconditions, output contracts, suitability, gaps, and evidence.",
            true,
            5,
            "Use before choosing or invoking a command.",
        ),
        FileSpec::new(
            "command-index.md",
            FileKind::CommandIndexReport,
            "Human-readable command inventory derived from command-index.json.",
            false,
            6,
            "Use for quick command-surface review.",
        ),
        FileSpec::new(
            "shape.json",
            FileKind::Shape,
            "Raw inferred command tree, flags, positionals, output contracts, gaps, confidence, and evidence pointers.",
            true,
            7,
            "Use when command-index.json does not contain enough inference detail.",
        ),
        FileSpec::new(
            "evidence.jsonl",
            FileKind::Evidence,
            "Append-only runtime event log.",
            true,
            8,
            "Use to prove what CLIARE observed at runtime.",
        ),
        FileSpec::new(
            "report.md",
            FileKind::ScoreReport,
            "Human-readable scorecard report.",
            false,
            9,
            "Use when explaining the score to maintainers.",
        ),
        FileSpec::new(
            "findings.sarif",
            FileKind::Sarif,
            "SARIF findings for code-scanning style consumers.",
            false,
            20,
            "Use for CI/code-scanning integrations.",
        ),
        FileSpec::new(
            "junit.xml",
            FileKind::Junit,
            "JUnit summary for CI systems.",
            false,
            21,
            "Use for test-report integrations.",
        ),
        FileSpec::new(
            "measure-cache.json",
            FileKind::Cache,
            "Fingerprint and profile cache manifest.",
            false,
            30,
            "Use to understand why a run was reused or recomputed.",
        ),
        FileSpec::new(
            "jobs/current",
            FileKind::JobPointer,
            "Pointer to the latest foreground or detached measurement job.",
            false,
            31,
            "Use with cliare jobs status --out <dir>.",
        ),
        FileSpec::new(
            "README.md",
            FileKind::Guide,
            "Artifact-directory guide generated by CLIARE.",
            false,
            40,
            "Use for command snippets and artifact orientation.",
        ),
        FileSpec::new(
            "AGENT_SKILL.md",
            FileKind::AgentSkill,
            "Agent workflow for artifact triage and evidence lookup.",
            false,
            41,
            "Use to guide an agent reviewing this artifact directory.",
        ),
    ]
}

fn benchmark_specs() -> Vec<FileSpec> {
    vec![
        FileSpec::new(
            "benchmark.json",
            FileKind::BenchmarkReport,
            "Aggregate benchmark corpus report.",
            true,
            1,
            "Start here to understand corpus totals and target status.",
        ),
        FileSpec::new(
            "benchmark.md",
            FileKind::BenchmarkReport,
            "Human-readable benchmark corpus report.",
            true,
            2,
            "Use for benchmark review and sharing.",
        ),
        FileSpec::new(
            "README.md",
            FileKind::Guide,
            "Benchmark artifact guide generated by CLIARE.",
            false,
            10,
            "Use for corpus navigation.",
        ),
        FileSpec::new(
            "AGENT_SKILL.md",
            FileKind::AgentSkill,
            "Agent workflow for benchmark artifact review.",
            false,
            11,
            "Use to guide an agent reviewing this benchmark directory.",
        ),
    ]
}

fn context_suite_specs() -> Vec<FileSpec> {
    vec![
        FileSpec::new(
            "context-suite.json",
            FileKind::ContextSuite,
            "Machine-readable comparison of persisted runtime contexts.",
            true,
            1,
            "Start here to see which contexts exist and how scores differ.",
        ),
        FileSpec::new(
            "context-compare.md",
            FileKind::ContextCompare,
            "Human-readable context comparison table.",
            false,
            2,
            "Use for a quick review of context-specific scores and preconditions.",
        ),
        FileSpec::new(
            "contexts",
            FileKind::ContextsDirectory,
            "Directory containing one complete measurement artifact bundle per runtime context.",
            true,
            3,
            "Choose a context, then inspect its scorecard, persona report, command index, and evidence.",
        ),
    ]
}

fn common_specs() -> Vec<FileSpec> {
    vec![
        FileSpec::new(
            "README.md",
            FileKind::Guide,
            "Directory guide if present.",
            false,
            20,
            "Use for local context.",
        ),
        FileSpec::new(
            "AGENT_SKILL.md",
            FileKind::AgentSkill,
            "Agent workflow if present.",
            false,
            21,
            "Use for agent-specific instructions.",
        ),
    ]
}

async fn dynamic_artifact_paths(
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

fn dynamic_file_spec(path: String) -> FileSpec {
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

async fn inspect_entry(folder: &Path, spec: FileSpec) -> ArtifactFile {
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

async fn read_json_value(path: &Path) -> std::result::Result<Value, String> {
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

async fn scorecard_summary(folder: &Path) -> Option<ScorecardSummary> {
    let value = read_json_value(&folder.join("scorecard.json")).await.ok()?;
    let score = value.get("score")?;
    let coverage = value.get("coverage").unwrap_or(&Value::Null);
    Some(ScorecardSummary {
        total: score.get("total").and_then(Value::as_f64),
        status: score
            .get("status")
            .and_then(Value::as_str)
            .map(str::to_owned),
        model: score
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_owned),
        probes_completed: coverage.get("probes_completed").and_then(Value::as_u64),
        max_probes: coverage.get("max_probes").and_then(Value::as_u64),
        traversal_complete: coverage.get("traversal_complete").and_then(Value::as_bool),
        budget_exhausted: coverage.get("budget_exhausted").and_then(Value::as_bool),
    })
}

async fn command_index_summary(folder: &Path) -> Option<CommandIndexSummary> {
    let value = read_json_value(&folder.join("command-index.json"))
        .await
        .ok()?;
    let commands = value.get("commands")?.as_array()?;
    let mut runtime_states = BTreeMap::new();
    let mut suitability = BTreeMap::new();
    let mut preconditioned = 0_u64;
    for command in commands {
        count_field(command, "runtime_state", &mut runtime_states);
        count_field(command, "agent_suitability", &mut suitability);
        if command
            .get("preconditions")
            .and_then(Value::as_array)
            .is_some_and(|values| !values.is_empty())
        {
            preconditioned += 1;
        }
    }
    Some(CommandIndexSummary {
        commands_total: commands.len() as u64,
        runtime_states,
        suitability,
        preconditioned,
    })
}

async fn issues_summary(folder: &Path) -> Option<IssuesSummary> {
    let value = read_json_value(&folder.join("issues.json")).await.ok()?;
    let issues = value.get("issues")?.as_array()?;
    let mut severity = BTreeMap::new();
    let mut confidence = BTreeMap::new();
    for issue in issues {
        count_field(issue, "severity", &mut severity);
        count_field(issue, "confidence", &mut confidence);
    }
    Some(IssuesSummary {
        issues_total: issues.len() as u64,
        severity,
        confidence,
    })
}

async fn benchmark_summary(folder: &Path) -> Option<BenchmarkSummary> {
    let value = read_json_value(&folder.join("benchmark.json")).await.ok()?;
    let totals = value.get("totals")?;
    Some(BenchmarkSummary {
        targets: totals.get("targets").and_then(Value::as_u64),
        measured: totals.get("measured").and_then(Value::as_u64),
        skipped: totals.get("skipped").and_then(Value::as_u64),
        failed: totals.get("failed").and_then(Value::as_u64),
        passed: totals.get("passed").and_then(Value::as_bool),
    })
}

async fn contexts_summary(folder: &Path) -> Option<ContextSuiteSummary> {
    let contexts = context::persisted_contexts(folder).await.ok()?;
    if contexts.is_empty() {
        return None;
    }
    Some(ContextSuiteSummary {
        contexts_total: contexts.len() as u64,
        contexts: contexts
            .into_iter()
            .map(|context| ContextSummaryItem {
                name: context.name,
                profile: context.profile.map(|profile| profile.label().to_owned()),
                artifact_dir: relative_to(folder, &context.artifact_dir),
            })
            .collect(),
    })
}

async fn job_summary(folder: &Path) -> Option<JobSummary> {
    let summary = crate::jobs::jobs(JobsArgs {
        command: JobsCommand::Status(JobsStatusArgs {
            out: folder.to_path_buf(),
            context: None,
        }),
    })
    .await
    .ok()?;
    summary.job_id.as_ref()?;
    Some(JobSummary {
        status: summary.status.label().to_owned(),
        job_id: summary.job_id,
        progress_log: summary.progress_log.map(|path| relative_to(folder, &path)),
        stdout_log: summary.stdout_log.map(|path| relative_to(folder, &path)),
        stderr_log: summary.stderr_log.map(|path| relative_to(folder, &path)),
        last_progress: summary.last_progress,
        last_error: summary.last_error,
    })
}

fn count_field(value: &Value, field: &str, counts: &mut BTreeMap<String, u64>) {
    if let Some(label) = value.get(field).and_then(Value::as_str) {
        *counts.entry(label.to_owned()).or_default() += 1;
    }
}

fn health(missing_required: &[String], summaries: &ArtifactSummaries) -> ArtifactHealth {
    let mut warnings = Vec::new();
    if !missing_required.is_empty() {
        warnings.push(format!(
            "{} required artifact(s) are missing",
            missing_required.len()
        ));
    }
    if summaries.scorecard.is_none()
        && summaries.benchmark.is_none()
        && summaries.contexts.is_none()
    {
        warnings.push("no scorecard.json or benchmark.json summary could be parsed".to_owned());
    }
    if summaries
        .job
        .as_ref()
        .is_some_and(|job| job.status == "failed")
    {
        warnings.push("latest measurement job failed".to_owned());
    }
    ArtifactHealth {
        status: if warnings.is_empty() {
            "ok".to_owned()
        } else {
            "attention".to_owned()
        },
        warnings,
    }
}

fn navigation_plan(kind: ArtifactKind) -> Vec<NavigationStep> {
    match kind {
        ArtifactKind::Measurement | ArtifactKind::Mixed => vec![
            step(
                1,
                "scorecard.json",
                "Read posture, score, coverage pressure, and provenance.",
            ),
            step(2, "issues.json", "Use as the canonical remediation queue."),
            step(
                3,
                "command-index.json",
                "Choose commands using runtime state, preconditions, parameters, and suitability.",
            ),
            step(
                4,
                "persona-<persona>.md",
                "Read the role-specific brief before drilling down.",
            ),
            step(
                5,
                "evidence.jsonl",
                "Verify claims by event id before making strong conclusions.",
            ),
            step(
                6,
                "jobs/current",
                "Check whether the latest measurement is still running or failed.",
            ),
        ],
        ArtifactKind::Benchmark => vec![
            step(
                1,
                "benchmark.json",
                "Read corpus totals, calibration status, and target summaries.",
            ),
            step(
                2,
                "benchmark.md",
                "Use the human-readable benchmark report.",
            ),
            step(
                3,
                "<target>/scorecard.json",
                "Inspect individual target measurements.",
            ),
            step(
                4,
                "<target>/command-index.json",
                "Inspect target command surfaces.",
            ),
        ],
        ArtifactKind::ContextSuite => vec![
            step(
                1,
                "context-suite.json",
                "Read persisted contexts, scores, preconditions, and artifact directories.",
            ),
            step(
                2,
                "context-compare.md",
                "Use the human-readable comparison table.",
            ),
            step(
                3,
                "contexts/<name>/persona-maintainer.md",
                "Open the persona packet inside the context you want to review.",
            ),
            step(
                4,
                "contexts/<name>/command-index.json",
                "Use the command index for context-specific harness navigation.",
            ),
        ],
        ArtifactKind::Unknown => vec![
            step(1, "README.md", "Read local context if present."),
            step(
                2,
                "artifact-map.json",
                "Use the generated map after running describe --write.",
            ),
        ],
    }
}

fn step(rank: u8, path: &str, purpose: &str) -> NavigationStep {
    NavigationStep {
        rank,
        path: path.to_owned(),
        purpose: purpose.to_owned(),
    }
}

fn render_markdown(map: &ArtifactMap) -> String {
    let mut out = String::new();
    out.push_str("# CLIARE Artifact Map\n\n");
    out.push_str(&format!("Folder: `{}`\n\n", escape_md(&map.folder)));
    out.push_str(&format!("Kind: `{}`\n\n", map.artifact_kind.label()));
    out.push_str(&format!("Generated: `{}`\n\n", map.generated_at));

    out.push_str("## Health\n\n");
    out.push_str(&format!("Status: `{}`\n\n", map.health.status));
    if !map.health.warnings.is_empty() {
        for warning in &map.health.warnings {
            out.push_str(&format!("- {}\n", escape_md(warning)));
        }
        out.push('\n');
    }

    out.push_str("## Navigation\n\n");
    out.push_str("| Step | Artifact | Purpose |\n|---:|---|---|\n");
    for step in &map.navigation {
        out.push_str(&format!(
            "| {} | `{}` | {} |\n",
            step.rank,
            escape_md(&step.path),
            escape_md(&step.purpose)
        ));
    }
    out.push('\n');

    render_summaries(map, &mut out);

    out.push_str("## Files\n\n");
    out.push_str("| Artifact | Status | Kind | Required | Schema/Records | Agent use |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for file in &map.files {
        let schema = file
            .schema_version
            .as_deref()
            .map(str::to_owned)
            .or_else(|| file.records.map(|records| format!("{records} records")))
            .or_else(|| file.parse_status.clone())
            .unwrap_or_else(|| "-".to_owned());
        out.push_str(&format!(
            "| `{}` | {} | `{}` | {} | {} | {} |\n",
            escape_md(&file.path),
            if file.exists { "present" } else { "missing" },
            file.kind.label(),
            if file.required { "yes" } else { "no" },
            escape_md(&schema),
            escape_md(&file.agent_use)
        ));
    }
    out.push('\n');

    if !map.missing_required.is_empty() {
        out.push_str("## Missing Required Artifacts\n\n");
        for path in &map.missing_required {
            out.push_str(&format!("- `{}`\n", escape_md(path)));
        }
        out.push('\n');
    }

    if !map.written_files.is_empty() {
        out.push_str("## Written Files\n\n");
        for path in &map.written_files {
            out.push_str(&format!("- `{}`\n", escape_md(path)));
        }
        out.push('\n');
    }

    out
}

fn render_summaries(map: &ArtifactMap, out: &mut String) {
    out.push_str("## Summary\n\n");
    if let Some(scorecard) = &map.summaries.scorecard {
        out.push_str(&format!(
            "- Score: {}\n",
            scorecard
                .total
                .map(|score| format!("{score:.0}/100"))
                .unwrap_or_else(|| "unknown".to_owned())
        ));
        out.push_str(&format!(
            "- Score status: `{}`\n",
            scorecard.status.as_deref().unwrap_or("unknown")
        ));
        out.push_str(&format!(
            "- Probes: {} / {}\n",
            optional_u64(scorecard.probes_completed),
            optional_u64(scorecard.max_probes)
        ));
        out.push_str(&format!(
            "- Traversal complete: `{}`\n",
            optional_bool(scorecard.traversal_complete)
        ));
    }
    if let Some(commands) = &map.summaries.command_index {
        out.push_str(&format!(
            "- Commands indexed: {}\n",
            commands.commands_total
        ));
        out.push_str(&format!(
            "- Commands with preconditions: {}\n",
            commands.preconditioned
        ));
    }
    if let Some(issues) = &map.summaries.issues {
        out.push_str(&format!("- Issues: {}\n", issues.issues_total));
    }
    if let Some(benchmark) = &map.summaries.benchmark {
        out.push_str(&format!(
            "- Benchmark targets: measured {} / {}\n",
            optional_u64(benchmark.measured),
            optional_u64(benchmark.targets)
        ));
    }
    if let Some(contexts) = &map.summaries.contexts {
        out.push_str(&format!(
            "- Persisted contexts: {}\n",
            contexts.contexts_total
        ));
        for context in &contexts.contexts {
            out.push_str(&format!("  - `{}`", escape_md(&context.name)));
            if let Some(profile) = &context.profile {
                out.push_str(&format!(" (`{}`)", escape_md(profile)));
            }
            out.push_str(&format!(": `{}`\n", escape_md(&context.artifact_dir)));
        }
    }
    if let Some(job) = &map.summaries.job {
        out.push_str(&format!("- Latest job: `{}`", job.status));
        if let Some(job_id) = &job.job_id {
            out.push_str(&format!(" (`{}`)", escape_md(job_id)));
        }
        out.push('\n');
    }
    out.push('\n');
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "unknown".to_owned(), |value| value.to_string())
}

fn optional_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "unknown".to_owned(), |value| value.to_string())
}

fn escape_md(value: &str) -> String {
    value.replace('|', "\\|")
}

async fn write_json(path: &Path, map: &ArtifactMap) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(map).map_err(CliareError::SerializeArtifactMap)?;
    fs::write(path, bytes)
        .await
        .map_err(|source| CliareError::WriteArtifactMap {
            path: path.to_path_buf(),
            source,
        })
}

async fn write_markdown(path: &Path, markdown: &str) -> Result<()> {
    fs::write(path, markdown.as_bytes())
        .await
        .map_err(|source| CliareError::WriteArtifactMap {
            path: path.to_path_buf(),
            source,
        })
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn timestamp() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(CliareError::TimeFormat)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Measurement,
    Benchmark,
    ContextSuite,
    Mixed,
    Unknown,
}

impl ArtifactKind {
    fn label(self) -> &'static str {
        match self {
            Self::Measurement => "measurement",
            Self::Benchmark => "benchmark",
            Self::ContextSuite => "context_suite",
            Self::Mixed => "mixed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    AgentSkill,
    Additional,
    ArtifactMap,
    ArtifactMapReport,
    BenchmarkReport,
    Cache,
    CiSummary,
    CommandIndex,
    CommandIndexReport,
    ContextCompare,
    ContextSuite,
    ContextsDirectory,
    Directory,
    Evidence,
    Guide,
    IssueLedger,
    IssueReport,
    JobPointer,
    JobLog,
    JobStderr,
    JobStdout,
    Junit,
    PersonaOutcome,
    PersonaReport,
    Sarif,
    Sandbox,
    Scorecard,
    ScoreReport,
    Shape,
}

impl FileKind {
    fn label(self) -> &'static str {
        match self {
            Self::AgentSkill => "agent_skill",
            Self::Additional => "additional",
            Self::ArtifactMap => "artifact_map",
            Self::ArtifactMapReport => "artifact_map_report",
            Self::BenchmarkReport => "benchmark_report",
            Self::Cache => "cache",
            Self::CiSummary => "ci_summary",
            Self::CommandIndex => "command_index",
            Self::CommandIndexReport => "command_index_report",
            Self::ContextCompare => "context_compare",
            Self::ContextSuite => "context_suite",
            Self::ContextsDirectory => "contexts_directory",
            Self::Directory => "directory",
            Self::Evidence => "evidence",
            Self::Guide => "guide",
            Self::IssueLedger => "issue_ledger",
            Self::IssueReport => "issue_report",
            Self::JobPointer => "job_pointer",
            Self::JobLog => "job_log",
            Self::JobStderr => "job_stderr",
            Self::JobStdout => "job_stdout",
            Self::Junit => "junit",
            Self::PersonaOutcome => "persona_outcome",
            Self::PersonaReport => "persona_report",
            Self::Sarif => "sarif",
            Self::Sandbox => "sandbox",
            Self::Scorecard => "scorecard",
            Self::ScoreReport => "score_report",
            Self::Shape => "shape",
        }
    }
}

struct FileSpec {
    path: String,
    kind: FileKind,
    role: String,
    required: bool,
    navigation_rank: u8,
    agent_use: String,
}

impl FileSpec {
    fn new(
        path: impl Into<String>,
        kind: FileKind,
        role: impl Into<String>,
        required: bool,
        navigation_rank: u8,
        agent_use: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            kind,
            role: role.into(),
            required,
            navigation_rank,
            agent_use: agent_use.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMap {
    pub schema_version: String,
    pub cliare_version: String,
    pub generated_at: String,
    pub folder: String,
    pub artifact_kind: ArtifactKind,
    pub health: ArtifactHealth,
    pub navigation: Vec<NavigationStep>,
    pub summaries: ArtifactSummaries,
    pub missing_required: Vec<String>,
    pub files: Vec<ArtifactFile>,
    pub written_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactHealth {
    pub status: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationStep {
    pub rank: u8,
    pub path: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFile {
    pub path: String,
    pub kind: FileKind,
    pub role: String,
    pub required: bool,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub records: Option<u64>,
    pub schema_version: Option<String>,
    pub parse_status: Option<String>,
    pub navigation_rank: u8,
    pub agent_use: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSummaries {
    pub scorecard: Option<ScorecardSummary>,
    pub command_index: Option<CommandIndexSummary>,
    pub issues: Option<IssuesSummary>,
    pub benchmark: Option<BenchmarkSummary>,
    pub contexts: Option<ContextSuiteSummary>,
    pub job: Option<JobSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorecardSummary {
    pub total: Option<f64>,
    pub status: Option<String>,
    pub model: Option<String>,
    pub probes_completed: Option<u64>,
    pub max_probes: Option<u64>,
    pub traversal_complete: Option<bool>,
    pub budget_exhausted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIndexSummary {
    pub commands_total: u64,
    pub runtime_states: BTreeMap<String, u64>,
    pub suitability: BTreeMap<String, u64>,
    pub preconditioned: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuesSummary {
    pub issues_total: u64,
    pub severity: BTreeMap<String, u64>,
    pub confidence: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub targets: Option<u64>,
    pub measured: Option<u64>,
    pub skipped: Option<u64>,
    pub failed: Option<u64>,
    pub passed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSuiteSummary {
    pub contexts_total: u64,
    pub contexts: Vec<ContextSummaryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummaryItem {
    pub name: String,
    pub profile: Option<String>,
    pub artifact_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub status: String,
    pub job_id: Option<String>,
    pub progress_log: Option<String>,
    pub stdout_log: Option<String>,
    pub stderr_log: Option<String>,
    pub last_progress: Option<String>,
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::cli::{DescribeArgs, DescribeFormat};

    #[tokio::test]
    async fn describe_measurement_directory_writes_artifact_map() {
        let folder =
            std::env::temp_dir().join(format!("cliare-describe-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("creates fixture directory");
        fs::write(
            folder.join("scorecard.json"),
            r#"{
  "schema_version": "cliare.scorecard.v1",
  "score": {"total": 82, "status": "experimental_partial", "model": "cliare-score-v0"},
  "coverage": {"probes_completed": 7, "max_probes": 64, "traversal_complete": true, "budget_exhausted": false}
}"#,
        )
        .expect("writes scorecard");
        fs::write(
            folder.join("command-index.json"),
            r#"{"schema_version":"cliare.command-index.v1","commands":[{"runtime_state":"runtime_confirmed","agent_suitability":"ready","preconditions":[]}]}"#,
        )
        .expect("writes command index");
        fs::write(
            folder.join("issues.json"),
            r#"{"schema_version":"cliare.issues.v1","issues":[{"severity":"high","confidence":"observed"}]}"#,
        )
        .expect("writes issues");
        fs::write(
            folder.join("shape.json"),
            r#"{"schema_version":"cliare.command-shape.v1"}"#,
        )
        .expect("writes shape");
        fs::write(folder.join("evidence.jsonl"), "{}\n{}\n").expect("writes evidence");

        let summary = super::describe(DescribeArgs {
            folder: folder.clone(),
            context: None,
            format: DescribeFormat::Markdown,
            write: true,
        })
        .await
        .expect("describe succeeds");

        assert_eq!(summary.artifact_kind, super::ArtifactKind::Measurement);
        assert_eq!(summary.missing_required, 0);
        assert!(summary.terminal_summary().contains("CLIARE Artifact Map"));
        assert!(folder.join("artifact-map.json").is_file());
        assert!(folder.join("artifact-map.md").is_file());

        let _ = fs::remove_dir_all(folder);
    }

    #[tokio::test]
    async fn describe_context_suite_root_lists_persisted_contexts() {
        let folder = std::env::temp_dir().join(format!(
            "cliare-describe-context-suite-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        let clean = folder.join("contexts/clean");
        let local = folder.join("contexts/local-context");
        fs::create_dir_all(&clean).expect("creates clean context");
        fs::create_dir_all(&local).expect("creates local context");
        fs::write(folder.join("context-suite.json"), "{}").expect("writes suite");
        fs::write(folder.join("context-compare.md"), "# compare\n").expect("writes comparison");
        fs::write(clean.join("scorecard.json"), "{}").expect("writes clean scorecard");
        fs::write(local.join("scorecard.json"), "{}").expect("writes local scorecard");
        fs::write(
            clean.join("runtime-context.json"),
            r#"{"schema_version":"cliare.runtime-context.v1","profile":"clean","name":"clean","auth_state":"absent","local_context_state":"absent","fixture_state":"absent","network_state":"unknown","runtime_dependency_state":"unknown","cwd_policy":"isolated","workdir":null,"declared_by":"cli"}"#,
        )
        .expect("writes clean runtime context");
        fs::write(
            local.join("runtime-context.json"),
            r#"{"schema_version":"cliare.runtime-context.v1","profile":"local_context","name":"local-context","auth_state":"unknown","local_context_state":"present","fixture_state":"absent","network_state":"unknown","runtime_dependency_state":"unknown","cwd_policy":"provided","workdir":"/tmp/project","declared_by":"cli"}"#,
        )
        .expect("writes local runtime context");

        let summary = super::describe(DescribeArgs {
            folder: folder.clone(),
            context: None,
            format: DescribeFormat::Markdown,
            write: false,
        })
        .await
        .expect("describe succeeds");

        assert_eq!(summary.artifact_kind, super::ArtifactKind::ContextSuite);
        assert!(summary.terminal_summary().contains("Persisted contexts: 2"));
        assert!(summary.terminal_summary().contains("contexts/clean"));
        assert!(
            summary
                .terminal_summary()
                .contains("contexts/local-context")
        );

        let _ = fs::remove_dir_all(folder);
    }
}
