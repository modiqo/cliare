use super::{
    CandidateFlagValueKind, command_candidates, flag_candidates, help_matches_command_path,
    is_help_like, is_manpage_like, output_mode_candidates, usage_arguments, usage_command_path,
};
use crate::output::OutputMode;

#[test]
fn extracts_commands_from_generic_aligned_rows() {
    let text = "TOOLS:\n  workspace ls [--flat]    List workspaces\n  flow search <QUERY>       Search flows\n";
    let candidates = command_candidates(text, "rote");

    assert!(
        candidates
            .iter()
            .any(|item| item.path == ["workspace", "ls"])
    );
    assert!(
        candidates
            .iter()
            .any(|item| item.path == ["flow", "search"])
    );
}

#[test]
fn treats_framework_help_as_generic_layout() {
    let text = "Commands:\n  measure  Run probes\n\nOptions:\n  -h, --help     Print help\n";

    assert!(is_help_like(text));
    assert!(
        command_candidates(text, "cliare")
            .iter()
            .any(|item| item.path == ["measure"])
    );
    assert!(
        flag_candidates(text)
            .iter()
            .any(|item| item.name == "--help")
    );
}

#[test]
fn ignores_wrapped_prose_that_happens_to_align_like_columns() {
    let text = "DESCRIPTION\n       current branch  with the same name on the remote\n       be given        from the command line or configuration\n       default mode    is selected for ordinary users\n";
    let candidates = command_candidates(text, "git");

    assert!(candidates.is_empty());
}

#[test]
fn rejects_overlong_invocation_prefixes_from_prose() {
    let text = "DETAILS\n       updates remote refs using local refs  while sending objects\n       pushes all matching branches at once  when configured\n";
    let candidates = command_candidates(text, "git");

    assert!(candidates.is_empty());
}

#[test]
fn rejects_title_case_prose_as_command_tokens() {
    let text = "OPTIONS\n       Show colored output  depending on configuration\n       Use mailmap file     to map author names\n";
    let candidates = command_candidates(text, "git");

    assert!(candidates.is_empty());
}

#[test]
fn detects_backspace_formatted_manpage_output() {
    assert!(is_manpage_like(
        "N\x08NA\x08AM\x08ME\x08E\n       tool - docs\n"
    ));
    assert!(!is_manpage_like("Commands:\n  run  Run a command\n"));
}

#[test]
fn rejects_numeric_menu_entries_as_command_paths() {
    let text = "INTERACTIVE\n       1  status\n       2  update\n";
    let candidates = command_candidates(text, "git");

    assert!(candidates.is_empty());
}

#[test]
fn rejects_argument_tables_as_command_candidates() {
    let text = "ARGUMENTS\n  FILE  Path to archive file\n  NAME  Resource name\n";
    let candidates = command_candidates(text, "tool");

    assert!(candidates.is_empty());
}

#[test]
fn rejects_key_value_tables_as_command_candidates() {
    let text = "SETTABLE KEYS\n  base_url     url     http(s) required\n  tags         csv     comma-separated labels\n";
    let candidates = command_candidates(text, "tool");

    assert!(candidates.is_empty());
}

#[test]
fn keeps_uppercase_commands_in_command_sections() {
    let text = "COMMANDS\n  GET  Execute HTTP GET request\n";
    let candidates = command_candidates(text, "tool");

    assert!(candidates.iter().any(|item| item.path == ["GET"]));
}

#[test]
fn extracts_simple_comma_separated_aliases_as_sibling_commands() {
    let text = "Commands:\n  rm, remove    Remove an item\n";
    let candidates = command_candidates(text, "tool");

    assert!(candidates.iter().any(|item| item.path == ["rm"]));
    assert!(candidates.iter().any(|item| item.path == ["remove"]));
    assert!(!candidates.iter().any(|item| item.path == ["rm", "remove"]));
    assert!(
        candidates
            .iter()
            .any(|item| item.path == ["rm"] && item.aliases == ["remove"])
    );
}

#[test]
fn absolute_invocation_rows_keep_root_paths_and_skip_placeholders() {
    let text = "NEXT STEPS\n  rote adapter info <ID>      Show details\n  rote adapter new ID SPEC    Create an adapter\n";
    let candidates = command_candidates(text, "rote");

    assert!(
        candidates
            .iter()
            .any(|item| { item.path == ["adapter", "info"] && item.absolute })
    );
    assert!(
        candidates
            .iter()
            .any(|item| { item.path == ["adapter", "new"] && item.absolute })
    );
    assert!(
        !candidates
            .iter()
            .any(|item| item.path == ["adapter", "new", "ID"])
    );
    assert!(
        !candidates
            .iter()
            .any(|item| item.path == ["adapter", "new", "ID", "SPEC"])
    );
}

#[test]
fn extracts_usage_positionals_from_current_command() {
    let text = "Usage: tool project deploy <PROJECT> [ENV] [FILES]...\n";
    let current_path = vec!["project".to_owned(), "deploy".to_owned()];
    let arguments = usage_arguments(text, "tool", &current_path);

    assert!(
        arguments
            .iter()
            .any(|arg| arg.name == "project" && arg.required)
    );
    assert!(
        arguments
            .iter()
            .any(|arg| arg.name == "env" && !arg.required)
    );
    assert!(
        arguments
            .iter()
            .any(|arg| arg.name == "files" && arg.variadic)
    );
}

#[test]
fn extracts_usage_positionals_from_header_blocks_and_skips_sibling_usage() {
    let text = "USAGE\n  tool adapter new <ID> <SPEC> [OPTIONS]\n  tool adapter new-from-mcp <ID> <MCP_ENDPOINT>\n\nARGUMENTS\n  ID  Identifier\n";
    let current_path = vec!["adapter".to_owned(), "new".to_owned()];
    let arguments = usage_arguments(text, "tool", &current_path);

    assert!(arguments.iter().any(|arg| arg.name == "id" && arg.required));
    assert!(
        arguments
            .iter()
            .any(|arg| arg.name == "spec" && arg.required)
    );
    assert!(!arguments.iter().any(|arg| arg.name == "mcp_endpoint"));
}

#[test]
fn detects_when_help_usage_matches_the_probed_command_path() {
    let text = "tool adapter set - Mutate a key\n\nUSAGE\n  tool adapter set <ID> <KEY> <VALUE> [--json]\n";

    assert!(help_matches_command_path(
        text,
        "tool",
        &["adapter".to_owned(), "set".to_owned()]
    ));
    assert!(!help_matches_command_path(
        text,
        "tool",
        &[
            "adapter".to_owned(),
            "set".to_owned(),
            "base_url".to_owned()
        ]
    ));
}

#[test]
fn detects_multiline_usage_as_matching_command_path() {
    let text = "Manage Supabase physical backups\n\nUsage:\n  supabase backups [command]\n\nAvailable Commands:\n  list     Lists available physical backups\n  restore  Restore to a specific timestamp using PITR\n";

    assert!(help_matches_command_path(
        text,
        "supabase",
        &["backups".to_owned()]
    ));
    assert_eq!(
        usage_command_path(text, "supabase", &["backups".to_owned()]),
        Some(vec!["backups".to_owned()])
    );
}

#[test]
fn extracts_usage_command_scope_for_parent_help_echoes() {
    let text = "USAGE\n  tool adapter set <ID> <KEY> <VALUE> [--json]\n";
    let current_path = vec![
        "adapter".to_owned(),
        "set".to_owned(),
        "base_url".to_owned(),
    ];

    assert_eq!(
        usage_command_path(text, "tool", &current_path),
        Some(vec!["adapter".to_owned(), "set".to_owned()])
    );
}

#[test]
fn usage_positionals_skip_flag_value_placeholders() {
    let text = "Usage: tool guard --baseline <FILE> <TARGET> [--format <KIND>]\n";
    let current_path = vec!["guard".to_owned()];
    let arguments = usage_arguments(text, "tool", &current_path);

    assert!(arguments.iter().any(|arg| arg.name == "target"));
    assert!(!arguments.iter().any(|arg| arg.name == "file"));
    assert!(!arguments.iter().any(|arg| arg.name == "kind"));
}

#[test]
fn extracts_flag_value_kind_requiredness_and_repeatability() {
    let text = "Options:\n  -f, --format <KIND>       Output format\n  --color[=<WHEN>]          Optional color mode\n  --tag <TAG>...            Repeatable tag\n  --token <TOKEN>           Required authentication token\n  --dry-run                 Do not write changes\n";
    let flags = flag_candidates(text);

    let format = flags
        .iter()
        .find(|flag| flag.name == "--format")
        .expect("format flag");
    assert_eq!(format.short.as_deref(), Some("-f"));
    assert_eq!(format.value_kind, CandidateFlagValueKind::Required);
    assert_eq!(format.value_name.as_deref(), Some("kind"));

    let color = flags
        .iter()
        .find(|flag| flag.name == "--color")
        .expect("color flag");
    assert_eq!(color.value_kind, CandidateFlagValueKind::Optional);

    let tag = flags
        .iter()
        .find(|flag| flag.name == "--tag")
        .expect("tag flag");
    assert!(tag.repeatable);

    let token = flags
        .iter()
        .find(|flag| flag.name == "--token")
        .expect("token flag");
    assert!(token.required);

    let dry_run = flags
        .iter()
        .find(|flag| flag.name == "--dry-run")
        .expect("dry-run flag");
    assert_eq!(dry_run.value_kind, CandidateFlagValueKind::Boolean);
}

#[test]
fn extracts_output_mode_candidates_from_structured_flags() {
    let text = "Options:\n  --json             Emit JSON\n  --format <KIND>    Output format: json or table\n  --output <FILE>    Output file\n";
    let candidates = output_mode_candidates(text);

    assert!(candidates.iter().any(|candidate| {
        candidate.mode == OutputMode::Json && candidate.argv_fragment == ["--json"]
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate.mode == OutputMode::Json && candidate.argv_fragment == ["--format", "json"]
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate.mode == OutputMode::Table && candidate.argv_fragment == ["--format", "table"]
    }));
    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.flag_name == "--output")
    );
}

#[test]
fn extracts_output_modes_from_choice_lists_in_flag_invocation() {
    let text = "Flags:\n  -o, --output [ env | pretty | json | toml | yaml ]   output format of status variables (default pretty)\n";
    let candidates = output_mode_candidates(text);

    assert!(candidates.iter().any(|candidate| {
        candidate.mode == OutputMode::Json
            && candidate.flag_name == "--output"
            && candidate.argv_fragment == ["--output", "json"]
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate.mode == OutputMode::Yaml
            && candidate.flag_name == "--output"
            && candidate.argv_fragment == ["--output", "yaml"]
    }));
}

#[test]
fn extracts_json_field_selector_probe_values() {
    let text = "FLAGS\n  -q, --jq expression      Filter JSON output using a jq expression\n      --json fields        Output JSON with the specified fields\n  -t, --template string    Format JSON output using a Go template\n\nJSON FIELDS\n  assignees, author, body, closed, closedAt, comments, createdAt, id,\n  labels, milestone, number, state, title, updatedAt, url\n";
    let candidates = output_mode_candidates(text);

    assert!(candidates.iter().any(|candidate| {
        candidate.mode == OutputMode::Json
            && candidate.flag_name == "--json"
            && candidate.argv_fragment == ["--json", "assignees,author,body"]
    }));
    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.flag_name == "--jq")
    );
    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.flag_name == "--template")
    );
}

#[test]
fn ignores_json_file_defaults_as_output_modes() {
    let text = "Options:\n  --manifest <FILE>  Benchmark corpus manifest [default: benchmarks/local-corpus.json]\n  --output <FILE>    Output file [default: report.json]\n";
    let candidates = output_mode_candidates(text);

    assert!(candidates.is_empty());
}

#[test]
fn ignores_json_input_payload_flags_as_output_modes() {
    let text = "Options:\n  --config <FILE>       Load configuration from JSON file\n  --config-json <JSON>  Pass auth/headers/filters as JSON\n  --dry-run             Analyze without creating and output JSON\n";
    let candidates = output_mode_candidates(text);

    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.flag_name == "--config")
    );
    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.flag_name == "--config-json")
    );
    assert!(candidates.iter().any(|candidate| {
        candidate.mode == OutputMode::Json && candidate.argv_fragment == ["--dry-run"]
    }));
}

#[test]
fn ignores_help_text_that_mentions_another_output_flag() {
    let text = "Options:\n  --format <FORMAT>  Output format [default: text] [possible values: text, json]\n  --help             Print help. With --format json, emit a parseable metadata contract\n";
    let candidates = output_mode_candidates(text);

    assert!(candidates.iter().any(|candidate| {
        candidate.mode == OutputMode::Json && candidate.argv_fragment == ["--format", "json"]
    }));
    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.flag_name == "--help")
    );
    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.argv_fragment == ["--format", "plain"])
    );
}
