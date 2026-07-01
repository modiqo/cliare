pub use cliare_inference::score_model::ScoreDimension as Dimension;

mod artifacts;
mod calculator;
mod findings;
mod formulas;
mod labels;
mod metrics;
mod model;
mod report;
mod util;

#[cfg(test)]
mod tests;

pub use artifacts::write_score_artifacts;
pub use calculator::scorecard;
pub use model::{
    Coverage, DimensionScore, DimensionStatus, Finding, SandboxScoreContext, ScoreArtifactSummary,
    ScoreModel, ScoreRunContext, ScoreStatus, ScoreSummary, Scorecard, Severity,
    TraversalStopReason,
};

const SCHEMA_VERSION: &str = "cliare.scorecard.v1";
