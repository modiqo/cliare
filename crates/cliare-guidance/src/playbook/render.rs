use std::fmt::Write as _;

use cliare_cli::cli::PlaybookRole;
use cliare_report::report_format::escape_markdown;

use super::{PlaybookSection, RolePlaybook};

pub(super) fn render_markdown(playbook: &RolePlaybook) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# {}", playbook.title).expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(&mut text, "{}", escape_markdown(playbook.goal))
        .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "| Field | Value |\n|---|---|\n| Target | `{}` |\n| Artifact dir | `{}` |\n| Context | `{}` |",
        escape_markdown(&playbook.target),
        playbook.out.display(),
        playbook.context.as_deref().unwrap_or("none")
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    render_artifact_layout(&mut text, playbook);

    for section in &playbook.lifecycle {
        writeln!(
            &mut text,
            "## {}. {}",
            section.order,
            escape_markdown(section.title)
        )
        .expect("writing to string cannot fail");
        writeln!(&mut text).expect("writing to string cannot fail");
        writeln!(&mut text, "{}", escape_markdown(section.purpose))
            .expect("writing to string cannot fail");
        writeln!(&mut text).expect("writing to string cannot fail");
        if section.title == "Act" {
            render_triage_order(&mut text);
        }
        for command in &section.commands {
            writeln!(&mut text, "### {}", escape_markdown(command.title))
                .expect("writing to string cannot fail");
            writeln!(&mut text).expect("writing to string cannot fail");
            writeln!(&mut text, "{}", escape_markdown(command.why))
                .expect("writing to string cannot fail");
            writeln!(&mut text).expect("writing to string cannot fail");
            writeln!(&mut text, "```sh").expect("writing to string cannot fail");
            writeln!(&mut text, "{}", command.command).expect("writing to string cannot fail");
            writeln!(&mut text, "```").expect("writing to string cannot fail");
            writeln!(&mut text).expect("writing to string cannot fail");
        }
    }

    render_parameter_guide(&mut text, playbook);
    render_publish_artifacts(&mut text, playbook);
    render_completion_criteria(&mut text, playbook);

    text
}

pub(super) fn render_human(playbook: &RolePlaybook) -> String {
    if playbook.role != PlaybookRole::Maintainer.label() {
        return render_role_human(playbook);
    }

    let mut text = String::new();
    writeln!(text, "CLIARE maintainer walkthrough").expect("writing to string cannot fail");
    writeln!(text, "target: {}", playbook.target).expect("writing to string cannot fail");
    writeln!(text, "artifacts: {}", playbook.out.display()).expect("writing to string cannot fail");
    if let Some(context) = &playbook.context {
        writeln!(text, "context: {context}").expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Read this as a checklist. Run one measure command, wait for it to finish, inspect issues, then fix or disposition before rerunning."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Artifact rule").expect("writing to string cannot fail");
    writeln!(
        text,
        "  {} is this target's artifact root. It is relative to your current directory.",
        playbook.out.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "  Do not use bare .cliare when .cliare contains multiple target folders."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    render_human_step(
        &mut text,
        1,
        "Measure",
        "Use standard for normal work. Use deep for release or launch-quality review.",
        &[
            (
                "normal",
                required_command(playbook, "Measure", "Normal maintainer loop"),
            ),
            (
                "quick edit loop",
                required_command(playbook, "Measure", "Local edit loop"),
            ),
            (
                "deep release pass",
                required_command(playbook, "Measure", "Release-quality pass"),
            ),
            (
                "large CLI",
                required_command(playbook, "Measure", "Very large CLI"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        2,
        "For long runs",
        "Detach only when you do not want to block the terminal. Do not read reports until the job is complete.",
        &[
            (
                "start detached",
                required_command(playbook, "Measure", "Detached long run"),
            ),
            (
                "check status",
                required_command(playbook, "View", "Detached job status"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        3,
        "Inspect",
        "Start with the issue list, then open the maintainer report or focused evidence when a row needs explanation.",
        &[
            ("issues", required_command(playbook, "View", "Issue ledger")),
            (
                "maintainer report",
                required_command(playbook, "View", "Maintainer report"),
            ),
            (
                "output contracts",
                required_command(playbook, "View", "Output contract drilldown"),
            ),
            (
                "one issue with evidence",
                required_command(playbook, "View", "Issue evidence bundle"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        4,
        "Act",
        "Fix real CLI contract gaps first: output contracts, preconditions, command-specific help, diagnostics, and safety.",
        &[],
    );
    render_human_step(
        &mut text,
        5,
        "Disposition what is not a bug",
        "Use a disposition when the finding is intentional, fixture-gated, not applicable, false positive, deferred, or accepted risk.",
        &[
            (
                "intentional behavior",
                required_command(playbook, "Disposition", "Intentional behavior"),
            ),
            (
                "needs fixture",
                required_command(playbook, "Disposition", "Fixture-gated issue"),
            ),
            (
                "review queue",
                required_command(playbook, "Disposition", "Review dispositions"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        6,
        "Remeasure",
        "After fixes or dispositions, regenerate evidence and verify that repeated noise dropped.",
        &[
            (
                "rerun",
                required_command(playbook, "Remeasure", "Deep rerun"),
            ),
            (
                "write reports",
                required_command(playbook, "Remeasure", "Persist reports"),
            ),
            (
                "verify",
                required_command(playbook, "Remeasure", "Verify remaining issues"),
            ),
        ],
    );
    render_human_step(
        &mut text,
        7,
        "Gate and publish",
        "Use a guard once a baseline exists, then publish the command index and harness packet for agents.",
        &[
            (
                "CI guard",
                required_command(playbook, "Gate in CI", "Score guard"),
            ),
            (
                "artifact map",
                required_command(playbook, "Publish Agent Surface", "Artifact navigation"),
            ),
            (
                "agent harness packet",
                required_command(playbook, "Publish Agent Surface", "Harness packet"),
            ),
        ],
    );

    writeln!(text, "Rules of thumb").expect("writing to string cannot fail");
    writeln!(
        text,
        "  Increase --max-depth or --max-probes only when the report shows traversal pressure."
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "  Do not increase probe budget for auth, fixture, daemon, repo, network, or runtime-dependency preconditions."
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "  For authenticated behavior, measure the same artifact root with --context authenticated."
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "  Use --format markdown for a full document or --format json for automation."
    )
    .expect("writing to string cannot fail");

    text
}

pub(super) fn render_role_human(playbook: &RolePlaybook) -> String {
    let mut text = String::new();
    writeln!(text, "CLIARE {} walkthrough", playbook.role).expect("writing to string cannot fail");
    writeln!(text, "target: {}", playbook.target).expect("writing to string cannot fail");
    writeln!(text, "artifacts: {}", playbook.out.display()).expect("writing to string cannot fail");
    if let Some(context) = &playbook.context {
        writeln!(text, "context: {context}").expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Goal").expect("writing to string cannot fail");
    writeln!(text, "  {}", playbook.goal).expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Artifact rule").expect("writing to string cannot fail");
    writeln!(
        text,
        "  {} is this target's artifact root, relative to your current directory.",
        playbook.out.display()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    for section in &playbook.lifecycle {
        render_human_step_from_section(&mut text, section);
    }

    writeln!(text, "Completion criteria").expect("writing to string cannot fail");
    for item in &playbook.completion_criteria {
        writeln!(text, "  - {item}").expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Use --format markdown for the full document or --format json for automation."
    )
    .expect("writing to string cannot fail");

    text
}

pub(super) fn render_human_step_from_section(text: &mut String, section: &PlaybookSection) {
    writeln!(text, "{}. {}", section.order, section.title).expect("writing to string cannot fail");
    writeln!(text, "   {}", section.purpose).expect("writing to string cannot fail");
    if section.commands.is_empty() {
        writeln!(
            text,
            "   Review the generated artifacts and apply the guidance manually."
        )
        .expect("writing to string cannot fail");
    }
    for command in &section.commands {
        writeln!(text, "   {}:", command.title).expect("writing to string cannot fail");
        writeln!(text, "     {}", command.command).expect("writing to string cannot fail");
        writeln!(text, "     {}", command.why).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_human_step(
    text: &mut String,
    number: u8,
    title: &str,
    guidance: &str,
    commands: &[(&str, &str)],
) {
    writeln!(text, "{number}. {title}").expect("writing to string cannot fail");
    writeln!(text, "   {guidance}").expect("writing to string cannot fail");
    for (label, command) in commands {
        writeln!(text, "   {label}:").expect("writing to string cannot fail");
        writeln!(text, "     {command}").expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn required_command<'a>(
    playbook: &'a RolePlaybook,
    section_title: &str,
    command_title: &str,
) -> &'a str {
    playbook
        .lifecycle
        .iter()
        .find(|section| section.title == section_title)
        .and_then(|section| {
            section
                .commands
                .iter()
                .find(|command| command.title == command_title)
        })
        .map(|command| command.command.as_str())
        .unwrap_or("missing playbook command")
}

pub(super) fn render_artifact_layout(text: &mut String, playbook: &RolePlaybook) {
    writeln!(text, "## Artifact Directory").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for item in &playbook.artifact_layout {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_triage_order(text: &mut String) {
    writeln!(text, "Triage in this order:").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    let rows = [
        (
            "Output contracts",
            "parseable JSON/YAML, safe dry-run behavior, fixture paths",
        ),
        (
            "Preconditions",
            "auth, local context, daemon, network, runtime dependency, fixture requirements",
        ),
        (
            "Command discovery",
            "command-specific --help and stable usage syntax",
        ),
        ("Diagnostics", "invalid command and invalid flag recovery"),
        (
            "Safety",
            "discovery-time side effects and credential-like paths",
        ),
        (
            "Compatibility advisories",
            "optional conventions such as help <path>",
        ),
    ];
    for (index, (title, detail)) in rows.iter().enumerate() {
        writeln!(text, "{}. {}: {}.", index + 1, title, detail)
            .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_parameter_guide(text: &mut String, playbook: &RolePlaybook) {
    writeln!(text, "## Parameter Guide").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Most maintainers should choose only `quick`, `standard`, or `deep`. Change advanced parameters only when the report points to traversal pressure."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Parameter | Meaning | Use When |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|---|").expect("writing to string cannot fail");
    for parameter in &playbook.parameter_guide {
        writeln!(
            text,
            "| `{}` | {} | {} |",
            parameter.name,
            escape_markdown(parameter.meaning),
            escape_markdown(parameter.use_when)
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");

    writeln!(text, "Increase depth or probes when:").expect("writing to string cannot fail");
    for item in &playbook.increase_budget_when {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Do not increase budget when:").expect("writing to string cannot fail");
    for item in &playbook.do_not_increase_budget_when {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_publish_artifacts(text: &mut String, playbook: &RolePlaybook) {
    writeln!(text, "## Agent-Facing Artifacts").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Publish or attach these so agent harnesses can route deliberately instead of rediscovering syntax by trial and error:"
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for artifact in &playbook.publish_artifacts {
        writeln!(text, "- `{}`", artifact).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_completion_criteria(text: &mut String, playbook: &RolePlaybook) {
    writeln!(text, "## Completion Criteria").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for item in &playbook.completion_criteria {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
}
