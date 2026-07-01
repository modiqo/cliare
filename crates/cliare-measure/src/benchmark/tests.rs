use std::path::Path;

use super::paths::{is_path_like, sanitize_target_id};

#[test]
fn target_id_sanitization_keeps_paths_portable() {
    assert_eq!(sanitize_target_id("git"), "git");
    assert_eq!(sanitize_target_id("foo/bar baz"), "foo_bar_baz");
}

#[test]
fn path_like_detection_keeps_command_names_on_path() {
    assert!(!is_path_like(Path::new("git")));
    assert!(is_path_like(Path::new("./target/debug/cliare")));
    assert!(is_path_like(Path::new("../target/debug/cliare")));
    assert!(is_path_like(Path::new("/usr/bin/git")));
}
