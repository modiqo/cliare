use std::path::Path;

pub(super) fn shell_arg_path(path: &Path) -> String {
    let value = path.display().to_string();
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        value
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
