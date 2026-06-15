pub(crate) fn command_path_label(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(" ")
    }
}

pub(crate) fn shell_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

pub(crate) fn shell_words(words: &[String]) -> String {
    if words.is_empty() {
        "<none>".to_owned()
    } else {
        words.join(" ")
    }
}

pub(crate) fn escape_markdown(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

pub(crate) fn output_mode_label(mode: &str) -> String {
    match mode {
        "json" => "JSON".to_owned(),
        "yaml" => "YAML".to_owned(),
        "table" => "table".to_owned(),
        "plain" => "plain text".to_owned(),
        other => other.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_markdown, shell_arg};

    #[test]
    fn shell_arguments_quote_spaces() {
        assert_eq!(shell_arg("target"), "target");
        assert_eq!(shell_arg("my target"), "'my target'");
    }

    #[test]
    fn markdown_escape_keeps_tables_valid() {
        assert_eq!(escape_markdown("a|b\nc"), "a\\|b c");
    }
}
