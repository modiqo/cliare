use super::suite::ContextSuite;

pub(super) fn render_context_suite(suite: &ContextSuite) -> String {
    let mut markdown = String::new();
    markdown.push_str("# CLIARE Context Comparison\n\n");
    if let Some(target) = &suite.target {
        markdown.push_str(&format!("- Target: `{}`\n", target.requested));
        markdown.push_str(&format!("- Resolved: `{}`\n", target.resolved));
        markdown.push_str(&format!("- Binary SHA-256: `{}`\n", target.binary_sha256));
    }
    markdown.push_str(&format!("- Contexts: `{}`\n", suite.contexts.len()));
    let chain = if suite.precondition_chain.is_empty() {
        "none".to_owned()
    } else {
        suite.precondition_chain.join(" -> ")
    };
    markdown.push_str(&format!("- Precondition chain: `{chain}`\n\n"));
    markdown.push_str(
        "| Context | Profile | Score | Commands | Preconditions | Traversal | Artifact Dir |\n",
    );
    markdown.push_str("|---|---|---:|---:|---|---|---|\n");
    for entry in &suite.contexts {
        let preconditions = if entry.preconditions_observed.is_empty() {
            "none".to_owned()
        } else {
            entry.preconditions_observed.join(", ")
        };
        markdown.push_str(&format!(
            "| `{}` | `{}` | {:.1} | {}/{} | `{}` | `{}` | `{}` |\n",
            escape_markdown_table(&entry.name),
            entry.profile.label(),
            entry.score,
            entry.commands_runtime_confirmed,
            entry.commands_discovered,
            preconditions,
            if entry.traversal_complete {
                "complete"
            } else {
                "partial"
            },
            entry.artifact_dir.display()
        ));
    }
    markdown.push('\n');
    markdown
}

fn escape_markdown_table(value: &str) -> String {
    value.replace('|', "\\|")
}
