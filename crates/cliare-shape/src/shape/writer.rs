use std::path::Path;

use crate::observation::ShapeObservation;
use cliare_core::artifacts::{COMMAND_INDEX_JSON, COMMAND_INDEX_MD, SHAPE_JSON, write_atomic};
use cliare_core::error::{CliareError, Result};
use cliare_runtime::fingerprint::TargetFingerprint;

use super::build::infer_shape;
use super::index::command_index;
use super::markdown::render_command_index_markdown;

pub async fn write_shape(
    out_dir: &Path,
    target: TargetFingerprint,
    observations: &[ShapeObservation],
) -> Result<()> {
    let shape = infer_shape(target, observations);
    let index = command_index(&shape);

    let shape_path = out_dir.join(SHAPE_JSON);
    let shape_bytes = serde_json::to_vec_pretty(&shape).map_err(CliareError::SerializeShape)?;
    write_atomic(&shape_path, &shape_bytes)
        .await
        .map_err(|source| CliareError::WriteShape {
            path: shape_path,
            source,
        })?;

    let index_path = out_dir.join(COMMAND_INDEX_JSON);
    let index_bytes = serde_json::to_vec_pretty(&index).map_err(CliareError::SerializeShape)?;
    write_atomic(&index_path, &index_bytes)
        .await
        .map_err(|source| CliareError::WriteShape {
            path: index_path,
            source,
        })?;

    let index_markdown_path = out_dir.join(COMMAND_INDEX_MD);
    let index_markdown = render_command_index_markdown(&index);
    write_atomic(&index_markdown_path, index_markdown.as_bytes())
        .await
        .map_err(|source| CliareError::WriteShape {
            path: index_markdown_path,
            source,
        })
}
