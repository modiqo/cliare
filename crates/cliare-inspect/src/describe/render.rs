use super::model::ArtifactMap;

pub(super) fn render_markdown(map: &ArtifactMap) -> String {
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
