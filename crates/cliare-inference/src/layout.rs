mod commands;
mod document;
mod extract;
mod flags;
mod output_modes;
mod sections;
mod types;

#[cfg(test)]
mod tests;

const MAX_COMMAND_PATH_SEGMENTS: usize = 3;

pub use commands::command_candidates;
pub use document::{ExtractionProfile, HelpDocument, LayoutLine};
pub use extract::{
    extraction_profile, help_matches_command_path, is_help_like, is_manpage_like, usage_arguments,
    usage_command_path,
};
pub use flags::flag_candidates;
pub use output_modes::output_mode_candidates;
pub use types::{
    CandidateArgument, CandidateCommand, CandidateFlag, CandidateFlagSection,
    CandidateFlagValueKind, CandidateOutputMode, CandidateOutputModeScope,
};
