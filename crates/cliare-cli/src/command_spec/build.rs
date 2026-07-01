use clap::builder::{OsStr, ValueRange};
use clap::{Arg, ArgAction, Command, CommandFactory, ValueHint};

use crate::cli::Cli;

use super::model::{
    ArgActionSpec, ArgKind, ArgSpec, COMMAND_SPEC_SCHEMA_VERSION, CliMetadata, CommandNode,
    CommandSpec, METADATA_SCHEMA_VERSION, PossibleValueSpec, ValueArity, ValueHintSpec,
};

pub fn metadata() -> CliMetadata {
    let root_command = Cli::command();
    let root = CommandNode::from_command(&root_command, Vec::new());
    let commands = root
        .subcommands
        .iter()
        .map(|command| command.name.clone())
        .collect();

    CliMetadata {
        schema_version: METADATA_SCHEMA_VERSION,
        name: "cliare",
        version: env!("CARGO_PKG_VERSION"),
        formats: &["text", "json"],
        commands,
        command_spec: CommandSpec {
            schema_version: COMMAND_SPEC_SCHEMA_VERSION,
            binary: root.name.clone(),
            version: env!("CARGO_PKG_VERSION"),
            root,
        },
    }
}

impl CommandNode {
    fn from_command(command: &Command, parent_path: Vec<String>) -> Self {
        let name = command.get_name().to_owned();
        let mut path = parent_path;
        path.push(name.clone());

        let mut usage_command = command.clone();
        let usage = full_usage(&mut usage_command, &path);
        let subcommands = command
            .get_subcommands()
            .filter(|subcommand| !subcommand.is_hide_set())
            .map(|subcommand| Self::from_command(subcommand, path.clone()))
            .collect();

        Self {
            name,
            path,
            usage,
            about: styled_text(command.get_about()),
            long_about: styled_text(command.get_long_about()),
            visible_aliases: command
                .get_visible_aliases()
                .map(ToOwned::to_owned)
                .collect(),
            args: command
                .get_arguments()
                .filter(|arg| !arg.is_hide_set())
                .map(ArgSpec::from_arg)
                .collect(),
            subcommands,
        }
    }
}

impl ArgSpec {
    fn from_arg(arg: &Arg) -> Self {
        let clap_action = arg.get_action();
        let action = ArgActionSpec::from_action(clap_action);
        let value_arity = ValueArity::from_range(arg.get_num_args().unwrap_or_else(|| {
            if clap_action.takes_values() {
                ValueRange::SINGLE
            } else {
                ValueRange::EMPTY
            }
        }));
        let kind = ArgKind::from_arg(arg, action, value_arity);

        Self {
            id: arg.get_id().to_string(),
            kind,
            action,
            short: arg.get_short(),
            long: arg.get_long().map(ToOwned::to_owned),
            value_names: value_names(arg, value_arity),
            value_arity,
            required: arg.is_required_set(),
            global: arg.is_global_set(),
            default_values: os_strings(arg.get_default_values()),
            possible_values: possible_values(arg, value_arity),
            value_hint: ValueHintSpec::from_hint(arg.get_value_hint()),
            help: styled_text(arg.get_help()),
            long_help: styled_text(arg.get_long_help()),
        }
    }
}

impl ArgKind {
    fn from_arg(arg: &Arg, action: ArgActionSpec, arity: ValueArity) -> Self {
        if arg.is_positional() {
            Self::Positional
        } else if action.takes_values() || arity.max.is_some_and(|max| max > 0) {
            Self::Option
        } else {
            Self::Flag
        }
    }
}

impl ArgActionSpec {
    fn from_action(action: &ArgAction) -> Self {
        match action {
            ArgAction::Set => Self::Set,
            ArgAction::Append => Self::Append,
            ArgAction::SetTrue => Self::SetTrue,
            ArgAction::SetFalse => Self::SetFalse,
            ArgAction::Count => Self::Count,
            ArgAction::Help => Self::Help,
            ArgAction::HelpShort => Self::HelpShort,
            ArgAction::HelpLong => Self::HelpLong,
            ArgAction::Version => Self::Version,
            _ => Self::Set,
        }
    }

    fn takes_values(self) -> bool {
        matches!(self, Self::Set | Self::Append)
    }
}

impl ValueArity {
    fn from_range(range: ValueRange) -> Self {
        let max_values = range.max_values();
        Self {
            min: range.min_values(),
            max: (max_values != usize::MAX).then_some(max_values),
        }
    }
}

impl ValueHintSpec {
    fn from_hint(hint: ValueHint) -> Self {
        match hint {
            ValueHint::Unknown => Self::Unknown,
            ValueHint::Other => Self::Other,
            ValueHint::AnyPath => Self::AnyPath,
            ValueHint::FilePath => Self::FilePath,
            ValueHint::DirPath => Self::DirPath,
            ValueHint::ExecutablePath => Self::ExecutablePath,
            ValueHint::CommandName => Self::CommandName,
            ValueHint::CommandString => Self::CommandString,
            ValueHint::CommandWithArguments => Self::CommandWithArguments,
            ValueHint::Username => Self::Username,
            ValueHint::Hostname => Self::Hostname,
            ValueHint::Url => Self::Url,
            ValueHint::EmailAddress => Self::EmailAddress,
            _ => Self::Unknown,
        }
    }
}

fn styled_text(text: Option<&clap::builder::StyledStr>) -> Option<String> {
    text.map(ToString::to_string)
        .filter(|text| !text.is_empty())
}

fn os_strings(values: &[OsStr]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.as_os_str().to_string_lossy().into_owned())
        .collect()
}

fn full_usage(command: &mut Command, path: &[String]) -> String {
    let usage = command.render_usage().to_string();
    let current = format!("Usage: {}", command.get_name());
    let full = format!("Usage: {}", path.join(" "));
    usage.replacen(&current, &full, 1)
}

fn possible_values(arg: &Arg, arity: ValueArity) -> Vec<PossibleValueSpec> {
    if arg.is_hide_possible_values_set() || arity.max == Some(0) {
        return Vec::new();
    }

    arg.get_possible_values()
        .into_iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| PossibleValueSpec {
            value: value.get_name().to_owned(),
            help: styled_text(value.get_help()),
        })
        .collect()
}

fn value_names(arg: &Arg, arity: ValueArity) -> Vec<String> {
    if arity.max == Some(0) {
        return Vec::new();
    }

    arg.get_value_names()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}
