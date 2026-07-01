use std::collections::BTreeSet;

pub(super) fn unique_command_paths(paths: Vec<Vec<String>>) -> Vec<Vec<String>> {
    paths
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn unique_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
