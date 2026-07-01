use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeIntent {
    Help,
    Version,
    InvalidCommand,
    InvalidChild,
    InvalidFlag,
    OutputJson,
    OutputYaml,
    OutputTable,
    OutputPlain,
    OutputJsonHelp,
    OutputYamlHelp,
    OutputTableHelp,
    OutputPlainHelp,
}
