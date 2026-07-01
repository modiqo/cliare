use crate::layout::{CandidateArgument, HelpDocument};
use crate::layout_tokens::{clean_token, is_argument_like, is_command_token};

pub(crate) fn usage_argument_from_token(
    token: &str,
    line_index: usize,
) -> Option<CandidateArgument> {
    if token.starts_with('-')
        || token.contains("--")
        || !token.contains('<') && !token.contains('[')
    {
        return None;
    }
    let name = value_name_from_token(token)?;
    if is_generic_usage_placeholder(&name) {
        return None;
    }

    Some(CandidateArgument {
        name,
        required: !token.starts_with('['),
        variadic: token.contains("..."),
        evidence_detail: format!("usage line {}", line_index),
    })
}

pub(crate) fn value_name_from_token(token: &str) -> Option<String> {
    let start = token.find('<').or_else(|| token.find('['))?;
    let mut value = token[start..].trim_matches(|ch: char| {
        matches!(
            ch,
            '<' | '>' | '[' | ']' | '=' | ',' | ':' | ';' | '(' | ')' | '{' | '}' | '.'
        )
    });
    if let Some(stripped) = value.strip_prefix('=') {
        value = stripped;
    }
    let end = value.find(['>', ']', '.', '=', ',']).unwrap_or(value.len());
    let name = value[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_ascii_lowercase().replace('-', "_"))
    }
}

pub(crate) fn bare_value_name_from_token(token: &str) -> Option<String> {
    let cleaned = clean_token(token).trim_matches(|ch: char| matches!(ch, '[' | ']'));
    if cleaned.is_empty()
        || cleaned.starts_with('-')
        || cleaned == "|"
        || cleaned.eq_ignore_ascii_case("default")
    {
        return None;
    }
    if !cleaned
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return None;
    }
    Some(cleaned.to_ascii_lowercase().replace('-', "_"))
}

pub(crate) fn usage_syntaxes(document: &HelpDocument) -> Vec<(usize, String)> {
    let mut syntaxes = Vec::new();
    let mut index = 0_usize;

    while index < document.lines().len() {
        let line = &document.lines()[index];
        if let Some((prefix, usage)) = line.text.split_once(':') {
            if prefix.eq_ignore_ascii_case("usage") {
                let usage = usage.trim();
                if !usage.is_empty() {
                    syntaxes.push((line.index, usage.to_owned()));
                    index += 1;
                    continue;
                }
            } else {
                index += 1;
                continue;
            }
        }

        if !is_usage_heading(&line.text) {
            index += 1;
            continue;
        }

        index += 1;
        while let Some(next) = document.lines().get(index) {
            if next.text.is_empty() {
                break;
            }
            if next.indent <= line.indent && next.is_header_like() {
                break;
            }
            syntaxes.push((next.index, next.text.clone()));
            index += 1;
        }
    }

    syntaxes
}

pub(crate) fn command_path_from_usage(
    usage: &str,
    binary_name: &str,
    current_path: &[String],
) -> Option<Vec<String>> {
    let mut path = Vec::new();
    let mut tokens = usage.split_whitespace().map(clean_token).peekable();
    if tokens.peek().is_some_and(|token| *token == binary_name) {
        tokens.next();
    }
    for token in tokens {
        if token.starts_with('-') || is_argument_like(token) || !is_command_token(token) {
            break;
        }
        path.push(token.to_owned());
    }
    if path.is_empty() {
        return None;
    }
    if current_path.is_empty() || current_path.starts_with(&path) {
        Some(path)
    } else {
        None
    }
}

pub(crate) fn usage_matches_current_path(
    usage: &str,
    binary_name: &str,
    current_path: &[String],
) -> bool {
    let mut tokens = usage.split_whitespace().map(clean_token).peekable();
    if tokens.peek().is_some_and(|token| *token == binary_name) {
        tokens.next();
    }
    for segment in current_path {
        match tokens.peek() {
            Some(token) if token == segment => {
                tokens.next();
            }
            _ => return false,
        }
    }
    true
}

pub(crate) fn usage_remainder_tokens<'a>(
    usage: &'a str,
    binary_name: &str,
    current_path: &[String],
) -> Vec<&'a str> {
    let mut tokens = usage.split_whitespace().map(clean_token).peekable();
    if tokens.peek().is_some_and(|token| *token == binary_name) {
        tokens.next();
    }
    for segment in current_path {
        if tokens.peek().is_some_and(|token| *token == segment) {
            tokens.next();
        }
    }
    tokens.collect()
}

fn is_generic_usage_placeholder(name: &str) -> bool {
    matches!(
        name,
        "options" | "option" | "command" | "commands" | "subcommand" | "subcommands" | "args"
    )
}

fn is_usage_heading(text: &str) -> bool {
    text.trim_matches(|ch: char| matches!(ch, ':' | '-' | '='))
        .eq_ignore_ascii_case("usage")
}

#[cfg(test)]
mod tests {
    use super::{usage_argument_from_token, value_name_from_token};

    #[test]
    fn extracts_required_and_optional_usage_operands() {
        let required = usage_argument_from_token("<project-ref>", 3).unwrap();
        assert_eq!(required.name, "project_ref");
        assert!(required.required);

        let optional = usage_argument_from_token("[FILE...]", 4).unwrap();
        assert_eq!(optional.name, "file");
        assert!(!optional.required);
        assert!(optional.variadic);
    }

    #[test]
    fn ignores_generic_usage_placeholders() {
        assert!(usage_argument_from_token("[options]", 1).is_none());
        assert_eq!(
            value_name_from_token("--format=<FORMAT>"),
            Some("format".to_owned())
        );
    }
}
