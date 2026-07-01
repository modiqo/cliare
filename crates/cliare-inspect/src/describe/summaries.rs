use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use cliare_cli::cli::{JobsArgs, JobsCommand, JobsStatusArgs};
use cliare_context as context;

use super::inspect::read_json_value;
use super::model::{
    BenchmarkSummary, CommandIndexSummary, ContextSuiteSummary, ContextSummaryItem, IssuesSummary,
    JobSummary, ScorecardSummary,
};
use super::util::relative_to;

pub(super) async fn scorecard_summary(folder: &Path) -> Option<ScorecardSummary> {
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

pub(super) async fn command_index_summary(folder: &Path) -> Option<CommandIndexSummary> {
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

pub(super) async fn issues_summary(folder: &Path) -> Option<IssuesSummary> {
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

pub(super) async fn benchmark_summary(folder: &Path) -> Option<BenchmarkSummary> {
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

pub(super) async fn contexts_summary(folder: &Path) -> Option<ContextSuiteSummary> {
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

pub(super) async fn job_summary(folder: &Path) -> Option<JobSummary> {
    let summary = cliare_measure::jobs::jobs(JobsArgs {
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
