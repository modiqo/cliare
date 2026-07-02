use std::path::{Path, PathBuf};

use cliare_core::artifacts::{CONDITION_DICTIONARY_CSV, write_atomic};
use cliare_core::error::{CliareError, Result};

pub const CONDITION_DICTIONARY: &str = include_str!("condition_dictionary.csv");

pub async fn write_condition_dictionary(out_dir: &Path) -> Result<PathBuf> {
    let path = out_dir.join(CONDITION_DICTIONARY_CSV);
    write_atomic(&path, CONDITION_DICTIONARY.as_bytes())
        .await
        .map_err(|source| CliareError::WriteArtifactGuide {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::CONDITION_DICTIONARY;

    #[test]
    fn condition_dictionary_has_expected_decoder_rows() {
        assert!(CONDITION_DICTIONARY.starts_with(
            "\"namespace\",\"condition\",\"plain_english\",\"agent_interpretation\",\"developer_action\",\"example\""
        ));
        assert!(CONDITION_DICTIONARY.contains("\"issue_confidence\",\"blocked\""));
        assert!(CONDITION_DICTIONARY.contains("\"precondition_kind\",\"local_context_required\""));
        assert!(CONDITION_DICTIONARY.contains("\"shape_gap\",\"output_mode_parse_failed\""));
        assert!(
            CONDITION_DICTIONARY
                .contains("\"agent_navigation_capability\",\"canonical_help_coverage\"")
        );
    }
}
