use std::path::{Path, PathBuf};

pub(super) fn resolve_manifest_target(manifest_dir: &Path, target: &Path) -> PathBuf {
    if is_path_like(target) && target.is_relative() {
        manifest_dir.join(target)
    } else {
        target.to_path_buf()
    }
}

pub(super) fn is_path_like(path: &Path) -> bool {
    path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
        || path.components().count() > 1
}

pub(super) fn sanitize_target_id(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
