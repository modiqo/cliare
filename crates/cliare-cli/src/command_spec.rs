mod build;
mod model;
mod render;

#[cfg(test)]
mod tests;

pub use build::metadata;
pub use model::{
    ArgActionSpec, ArgKind, ArgSpec, CliMetadata, CommandNode, CommandSpec, PossibleValueSpec,
    ValueArity, ValueHintSpec,
};
pub use render::{metadata_help, metadata_text};
