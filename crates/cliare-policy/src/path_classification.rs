use std::path::Path;

const CREDENTIAL_PATH_TERMS: &[&str] = &[
    "token",
    "secret",
    "credential",
    "credentials",
    "apikey",
    "api_key",
    "keychain",
    "key",
];

pub fn credential_like_path(path: &Path) -> bool {
    credential_like_path_text(&path.display().to_string())
}

pub fn credential_like_path_text(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    CREDENTIAL_PATH_TERMS
        .iter()
        .any(|term| lower.contains(term))
}

#[cfg(test)]
mod tests {
    #[test]
    fn credential_like_paths_cover_common_secret_file_names() {
        assert!(super::credential_like_path_text(
            "home/.config/tool/token.json"
        ));
        assert!(super::credential_like_path_text("project/api_key.txt"));
        assert!(super::credential_like_path_text(
            "Library/Keychains/login.keychain-db"
        ));
        assert!(!super::credential_like_path_text("cache/help-output.json"));
    }
}
