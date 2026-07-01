use cliare_inference::precondition::PreconditionKind;

use super::labels::{
    escape_markdown_table, gap_kind_label, output_status_label, runtime_state_label,
    suitability_label,
};
use super::model::{
    CommandIndex, CommandIndexFlag, CommandIndexGap, CommandIndexOutputContract, CommandParameters,
    FlagValueKindShape, PositionalArgument,
};

pub(super) fn render_command_index_markdown(index: &CommandIndex) -> String {
    let mut lines = vec![
        "# CLIARE Command Index".to_owned(),
        String::new(),
        "Command-centric reference generated from `shape.json`. Use it as the first-pass index for selecting commands, understanding parameters, and identifying runtime constraints before invoking a CLI from an agent harness.".to_owned(),
        String::new(),
        format!("- Target: `{}`", index.target.requested.display()),
        format!("- Resolved: `{}`", index.target.resolved.display()),
        format!("- Commands: `{}`", index.summary.commands_total),
        format!("- Ready: `{}`", index.summary.ready),
        format!("- Conditional: `{}`", index.summary.conditional),
        format!("- Needs fixture: `{}`", index.summary.needs_fixture),
        format!("- Blocked: `{}`", index.summary.blocked),
        format!("- Candidate only: `{}`", index.summary.candidate),
        String::new(),
        "## Suitability Legend".to_owned(),
        String::new(),
        "| Suitability | Meaning | Harness treatment |".to_owned(),
        "|---|---|---|".to_owned(),
        "| `ready` | Runtime-confirmed command with no blocking gaps. | Candidate for automatic routing, subject to harness permissions and task policy. |".to_owned(),
        "| `conditional` | Runtime-confirmed or runtime-recognized command with a satisfiable condition such as auth, local context, unavailable help, unknown grammar, missing diagnostics, or output parse failure. | Expose when the harness can satisfy the condition or policy allows manual review; prefer safer alternatives for unattended actions. |".to_owned(),
        "| `needs_fixture` | Advertised contract or behavior needs safe operands, fixture data, or command-local validation before CLIARE can verify it. | Add fixtures or documented safe operands before routing automatically. |".to_owned(),
        "| `blocked` | Opaque, infrastructure-level, or currently unsatisfied runtime condition blocked safe confirmation or use. | Treat as unavailable until the runtime condition is provisioned or the diagnostic becomes explicit enough to classify. |".to_owned(),
        "| `candidate` | Inferred from help/layout evidence but not runtime-confirmed. | Keep out of automatic routing until deeper measurement or catalog metadata confirms it. |".to_owned(),
        String::new(),
        "## Commands".to_owned(),
        String::new(),
        "| Command | Suitability | Runtime | Parameters | Preconditions | Output | Gaps | Summary |".to_owned(),
        "|---|---|---|---|---|---|---|---|".to_owned(),
    ];

    for command in &index.commands {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | {} | {} | {} | {} | {} |",
            escape_markdown_table(&command.command),
            suitability_label(command.agent_suitability),
            runtime_state_label(command.runtime_state),
            escape_markdown_table(&parameters_summary(&command.parameters)),
            escape_markdown_table(&preconditions_summary(&command.preconditions)),
            escape_markdown_table(&output_summary(&command.output_contracts)),
            escape_markdown_table(&gaps_summary(&command.gaps)),
            escape_markdown_table(command.summary.as_deref().unwrap_or(""))
        ));
    }

    lines.push(String::new());
    lines.join("\n")
}

fn parameters_summary(parameters: &CommandParameters) -> String {
    let positionals = parameters
        .positionals
        .iter()
        .map(format_positional)
        .collect::<Vec<_>>();
    let flags = parameters.flags.iter().map(format_flag).collect::<Vec<_>>();

    match (positionals.is_empty(), flags.is_empty()) {
        (true, true) => "none".to_owned(),
        (false, true) => format!("args: {}", positionals.join(", ")),
        (true, false) => format!("flags: {}", flags.join(", ")),
        (false, false) => format!(
            "args: {}; flags: {}",
            positionals.join(", "),
            flags.join(", ")
        ),
    }
}

fn format_positional(positional: &PositionalArgument) -> String {
    let mut name = positional.name.clone();
    if positional.variadic {
        name.push_str("...");
    }
    if positional.required {
        format!("<{name}>")
    } else {
        format!("[{name}]")
    }
}

fn format_flag(flag: &CommandIndexFlag) -> String {
    let mut rendered = flag.name.clone();
    if let Some(value_name) = &flag.value_name {
        match flag.value_kind {
            FlagValueKindShape::Boolean => {}
            FlagValueKindShape::Required => {
                rendered.push(' ');
                rendered.push_str(value_name);
            }
            FlagValueKindShape::Optional => {
                rendered.push_str(" [");
                rendered.push_str(value_name);
                rendered.push(']');
            }
        }
    }
    if flag.repeatable {
        rendered.push_str("...");
    }
    rendered
}

fn preconditions_summary(preconditions: &[PreconditionKind]) -> String {
    if preconditions.is_empty() {
        "none".to_owned()
    } else {
        preconditions
            .iter()
            .map(|precondition| precondition.label())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn output_summary(contracts: &[CommandIndexOutputContract]) -> String {
    if contracts.is_empty() {
        return "none".to_owned();
    }
    contracts
        .iter()
        .map(|contract| {
            format!(
                "{}:{}",
                contract.mode.label(),
                output_status_label(contract.status)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn gaps_summary(gaps: &[CommandIndexGap]) -> String {
    if gaps.is_empty() {
        return "none".to_owned();
    }
    gaps.iter()
        .map(|gap| gap_kind_label(gap.kind))
        .collect::<Vec<_>>()
        .join(", ")
}
