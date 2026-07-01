use super::types::CandidateFlagSection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SectionKind {
    Command,
    NonCommand,
    Unknown,
}

pub(super) fn section_kind(text: &str) -> SectionKind {
    let normalized = text
        .trim_matches(|ch: char| matches!(ch, ':' | '-' | '='))
        .to_ascii_lowercase();
    let words = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    if words
        .iter()
        .any(|word| matches!(*word, "commands" | "command" | "subcommands" | "subcommand"))
    {
        return SectionKind::Command;
    }

    if words.iter().any(|word| {
        matches!(
            *word,
            "arguments"
                | "argument"
                | "parameters"
                | "parameter"
                | "options"
                | "option"
                | "flags"
                | "flag"
                | "examples"
                | "example"
                | "usage"
                | "environment"
                | "outputs"
                | "output"
                | "inputs"
                | "input"
                | "keys"
                | "key"
                | "values"
                | "value"
                | "fields"
                | "field"
                | "types"
                | "type"
                | "exit"
                | "codes"
                | "code"
                | "notes"
                | "note"
                | "details"
                | "location"
                | "locations"
                | "provider"
                | "providers"
        )
    }) {
        return SectionKind::NonCommand;
    }

    SectionKind::Unknown
}

pub(super) fn flag_section(text: &str) -> CandidateFlagSection {
    let normalized = text
        .trim_matches(|ch: char| matches!(ch, ':' | '-' | '='))
        .to_ascii_lowercase();
    let words = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let flag_like = words.iter().any(|word| {
        matches!(
            *word,
            "flags" | "flag" | "options" | "option" | "switches" | "switch"
        )
    });

    if !flag_like {
        return CandidateFlagSection::Other;
    }
    if words
        .iter()
        .any(|word| matches!(*word, "global" | "persistent" | "inherited" | "common"))
    {
        CandidateFlagSection::Global
    } else {
        CandidateFlagSection::Command
    }
}

pub(super) fn is_example_section(text: &str) -> bool {
    let normalized = text
        .trim_matches(|ch: char| matches!(ch, ':' | '-' | '='))
        .to_ascii_lowercase();
    normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| matches!(word, "example" | "examples"))
}
