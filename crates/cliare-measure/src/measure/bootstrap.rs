use crate::evidence::ProbeIntent;
use crate::fingerprint::TargetFingerprint;
use crate::planner::{bootstrap_invalid_command_token, bootstrap_invalid_flag_token};
use crate::process::ProbeSpec;

pub(super) fn bootstrap_probes(target: &TargetFingerprint) -> Vec<ProbeSpec> {
    let target_name = target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target");
    let invalid_command = bootstrap_invalid_command_token(target_name);
    let invalid_flag = bootstrap_invalid_flag_token(target_name);

    vec![
        ProbeSpec::new(["--help"], ProbeIntent::Help),
        ProbeSpec::new(["-h"], ProbeIntent::Help),
        ProbeSpec::new(["help"], ProbeIntent::Help),
        ProbeSpec::new(["--version"], ProbeIntent::Version),
        ProbeSpec::new(["version"], ProbeIntent::Version),
        ProbeSpec::from_vec(vec![invalid_command], ProbeIntent::InvalidCommand),
        ProbeSpec::from_vec(vec![invalid_flag], ProbeIntent::InvalidFlag),
    ]
}

pub(super) fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}

pub(super) fn invalid_token_seed(binary_name: &str) -> String {
    binary_name.replace('-', "_")
}
