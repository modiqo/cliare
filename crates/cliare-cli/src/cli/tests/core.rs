use super::support::*;

#[test]
fn clap_surface_exposes_measure_and_global_version() {
    let mut command = Cli::command();
    let help = command.render_long_help().to_string();

    assert!(help.contains("Usage: cliare"));
    assert!(help.contains("measure"));
    assert!(help.contains("jobs"));
    assert!(help.contains("guard"));
    assert!(help.contains("benchmark"));
    assert!(help.contains("eval"));
    assert!(help.contains("report"));
    assert!(help.contains("describe"));
    assert!(help.contains("skills"));
    assert!(help.contains("issues"));
    assert!(help.contains("surface"));
    assert!(help.contains("playbook"));
    assert!(help.contains("context"));
    assert!(help.contains("metadata"));
    assert!(help.contains("--version"));
}
