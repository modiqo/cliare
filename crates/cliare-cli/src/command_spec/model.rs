use serde::Serialize;

pub(super) const METADATA_SCHEMA_VERSION: &str = "cliare.metadata.v1";
pub(super) const COMMAND_SPEC_SCHEMA_VERSION: &str = "cliare.command-spec.v1";

#[derive(Debug, Serialize)]
pub struct CliMetadata {
    pub(super) schema_version: &'static str,
    pub(super) name: &'static str,
    pub(super) version: &'static str,
    pub(super) formats: &'static [&'static str],
    pub(super) commands: Vec<String>,
    pub(super) command_spec: CommandSpec,
}

#[derive(Debug, Serialize)]
pub struct CommandSpec {
    pub(super) schema_version: &'static str,
    pub(super) binary: String,
    pub(super) version: &'static str,
    pub(super) root: CommandNode,
}

#[derive(Debug, Serialize)]
pub struct CommandNode {
    pub(super) name: String,
    pub(super) path: Vec<String>,
    pub(super) usage: String,
    pub(super) about: Option<String>,
    pub(super) long_about: Option<String>,
    pub(super) visible_aliases: Vec<String>,
    pub(super) args: Vec<ArgSpec>,
    pub(super) subcommands: Vec<CommandNode>,
}

#[derive(Debug, Serialize)]
pub struct ArgSpec {
    pub(super) id: String,
    pub(super) kind: ArgKind,
    pub(super) action: ArgActionSpec,
    pub(super) short: Option<char>,
    pub(super) long: Option<String>,
    pub(super) value_names: Vec<String>,
    pub(super) value_arity: ValueArity,
    pub(super) required: bool,
    pub(super) global: bool,
    pub(super) default_values: Vec<String>,
    pub(super) possible_values: Vec<PossibleValueSpec>,
    pub(super) value_hint: ValueHintSpec,
    pub(super) help: Option<String>,
    pub(super) long_help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgKind {
    Flag,
    Option,
    Positional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgActionSpec {
    Set,
    Append,
    SetTrue,
    SetFalse,
    Count,
    Help,
    HelpShort,
    HelpLong,
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ValueArity {
    pub(super) min: usize,
    pub(super) max: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct PossibleValueSpec {
    pub(super) value: String,
    pub(super) help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueHintSpec {
    Unknown,
    Other,
    AnyPath,
    FilePath,
    DirPath,
    ExecutablePath,
    CommandName,
    CommandString,
    CommandWithArguments,
    Username,
    Hostname,
    Url,
    EmailAddress,
}
