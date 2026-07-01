use std::fmt::Write as _;

use clap::CommandFactory;

use crate::cli::Cli;

use super::build::metadata;
use super::model::{ArgKind, ArgSpec, CliMetadata, CommandNode};

pub fn metadata_help() -> String {
    let mut command = Cli::command();
    match command.find_subcommand_mut("metadata") {
        Some(metadata) => {
            metadata.set_bin_name("cliare metadata");
            metadata.render_long_help().to_string()
        }
        None => "Print CLIARE implementation metadata\n".to_owned(),
    }
}

pub fn metadata_text() -> String {
    let metadata = metadata();
    let mut text = String::new();
    writeln!(&mut text, "{} {}", metadata.name, metadata.version)
        .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "Command spec: {}",
        metadata.command_spec.schema_version
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "Full structured spec: cliare metadata --format json"
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(&mut text, "Commands").expect("writing to string cannot fail");
    render_commands(&mut text, &metadata);
    text
}

fn render_commands(text: &mut String, metadata: &CliMetadata) {
    for command in &metadata.command_spec.root.subcommands {
        render_command_text(text, command, 0);
    }
}

fn render_command_text(text: &mut String, command: &CommandNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let about = command.about.as_deref().unwrap_or("");
    if about.is_empty() {
        writeln!(text, "{}- {}", indent, command.path.join(" "))
            .expect("writing to string cannot fail");
    } else {
        writeln!(text, "{}- {} - {}", indent, command.path.join(" "), about)
            .expect("writing to string cannot fail");
    }
    writeln!(text, "{}  {}", indent, command.usage).expect("writing to string cannot fail");
    if !command.args.is_empty() {
        let option_labels = command
            .args
            .iter()
            .filter(|arg| matches!(arg.kind, ArgKind::Option | ArgKind::Flag))
            .map(arg_label)
            .collect::<Vec<_>>();
        if !option_labels.is_empty() {
            writeln!(text, "{}  options: {}", indent, option_labels.join(", "))
                .expect("writing to string cannot fail");
        }
    }
    for subcommand in &command.subcommands {
        render_command_text(text, subcommand, depth + 1);
    }
}

fn arg_label(arg: &ArgSpec) -> String {
    match (&arg.short, &arg.long) {
        (Some(short), Some(long)) => format!("-{short}, --{long}"),
        (None, Some(long)) => format!("--{long}"),
        (Some(short), None) => format!("-{short}"),
        (None, None) => arg.id.clone(),
    }
}
