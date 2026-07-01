mod build;
mod gaps;
mod index;
mod labels;
mod markdown;
mod model;
mod writer;

#[cfg(test)]
mod tests;

const SCHEMA_VERSION: &str = "cliare.command-shape.v1";
const COMMAND_INDEX_SCHEMA_VERSION: &str = "cliare.command-index.v1";
const INFERENCE_MODEL: &str = "cliare-generic-claims-v0";

pub use build::infer_shape;
pub use index::infer_command_index;
pub use model::{
    AgentSuitability, CommandCandidate, CommandIndex, CommandIndexEntry, CommandIndexFlag,
    CommandIndexGap, CommandIndexOutputContract, CommandIndexSummary, CommandParameters,
    CommandRuntimeState, CommandShape, FlagCandidate, FlagValueKindShape, Gap, GapKind,
    InferenceModel, OutputContractCandidate, OutputContractStatus, PositionalArgument,
};
pub use writer::write_shape;
