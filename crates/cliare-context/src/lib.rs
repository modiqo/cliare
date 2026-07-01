pub const RUNTIME_CONTEXT_SCHEMA_VERSION: &str = "cliare.runtime-context.v1";
const CONTEXT_SUITE_SCHEMA_VERSION: &str = "cliare.context-suite.v1";

mod artifact;
mod cli;
mod render;
mod runtime;
mod suite;

#[cfg(test)]
mod tests;

pub use artifact::{
    context_artifact_dir, contexts_dir, is_context_suite_root, measurement_dir, persisted_contexts,
    resolve_measurement_dir, write_runtime_context,
};
pub use cli::{ContextArgs, ContextCommand, ContextCompareArgs, ContextCompareFormat};
pub use runtime::{
    PersistedContext, RuntimeContext, RuntimeContextCwdPolicy, RuntimeContextDeclaration,
    RuntimeContextInput, RuntimeContextProfile, RuntimeContextState,
};
pub use suite::{ContextSuite, ContextSummary, context, refresh_context_suite};
