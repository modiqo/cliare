pub(crate) fn split_columns(line: &str) -> Vec<String> {
    let mut columns = Vec::new();
    let mut current = String::new();
    let mut spaces = 0_usize;

    for ch in line.trim().chars() {
        if ch.is_whitespace() {
            spaces += 1;
            continue;
        }

        if spaces >= 2 && !current.trim().is_empty() {
            columns.push(current.trim().to_owned());
            current.clear();
        } else if spaces == 1 && !current.is_empty() {
            current.push(' ');
        }

        spaces = 0;
        current.push(ch);
    }

    if !current.trim().is_empty() {
        columns.push(current.trim().to_owned());
    }

    columns
}

pub(crate) fn clean_token(token: &str) -> &str {
    token.trim_matches(|ch: char| matches!(ch, ',' | ':' | ';' | '(' | ')' | '{' | '}'))
}

pub(crate) fn is_argument_like(token: &str) -> bool {
    token.starts_with('<')
        || token.starts_with('[')
        || token.contains('<')
        || token.contains('[')
        || token.contains('=')
        || token == "..."
}

pub(crate) fn is_placeholder_like(token: &str) -> bool {
    let mut has_uppercase = false;
    let mut has_lowercase = false;
    let mut has_separator = false;

    for ch in token.chars() {
        if ch.is_ascii_uppercase() {
            has_uppercase = true;
        } else if ch.is_ascii_lowercase() {
            has_lowercase = true;
        } else if matches!(ch, '_' | '-') {
            has_separator = true;
        } else if !ch.is_ascii_digit() {
            return false;
        }
    }

    has_uppercase && !has_lowercase && (token.len() >= 2 || has_separator)
}

pub(crate) fn is_command_token(token: &str) -> bool {
    token
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '@'))
        && !is_title_case_word(token)
}

pub(crate) fn is_title_case_word(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_uppercase())
        && chars.any(|ch| ch.is_ascii_lowercase())
        && token.chars().all(|ch| ch.is_ascii_alphabetic())
}

pub(crate) fn short_flag_in(text: &str) -> Option<String> {
    text.split(|ch: char| ch == ',' || ch.is_whitespace())
        .map(clean_token)
        .find(|token| {
            token.starts_with('-')
                && !token.starts_with("--")
                && token.len() > 1
                && token.chars().skip(1).all(|ch| ch.is_ascii_alphabetic())
        })
        .map(str::to_owned)
}

pub(crate) fn long_flag_name(token: &str) -> Option<String> {
    let cleaned = clean_token(token);
    if !cleaned.starts_with("--") {
        return None;
    }
    let end = cleaned.find(['=', '[', '<']).unwrap_or(cleaned.len());
    let name = &cleaned[..end];
    if name.len() > 2 {
        Some(name.to_owned())
    } else {
        None
    }
}

pub(crate) fn starts_title_like(text: &str) -> bool {
    text.chars()
        .find(|ch| !ch.is_whitespace())
        .is_some_and(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    #[test]
    fn splits_aligned_columns_without_changing_single_space_text() {
        assert_eq!(
            super::split_columns("  deploy  Deploy the current app"),
            vec!["deploy", "Deploy the current app"]
        );
    }

    #[test]
    fn title_case_words_are_not_command_tokens() {
        assert!(!super::is_command_token("Deploy"));
        assert!(super::is_command_token("deploy-app"));
    }
}
