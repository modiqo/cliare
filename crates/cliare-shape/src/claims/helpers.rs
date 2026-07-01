use cliare_core::probe_intent::ProbeIntent;
use cliare_core::process_status::ProcessStatus;
use cliare_inference::layout;
use cliare_inference::output::OutputMode;
use cliare_inference::precondition::{self, PreconditionKind};

use crate::observation::ShapeObservation;

pub(super) fn successful_help_like(observation: &ShapeObservation) -> bool {
    matches!(
        &observation.process.status,
        ProcessStatus::Exited { code: Some(0) }
    ) && observation
        .process
        .stdout
        .text
        .as_deref()
        .is_some_and(layout::is_help_like)
}

pub(super) fn precondition_for_observation(
    observation: &ShapeObservation,
) -> Option<PreconditionKind> {
    precondition::classify_process(
        &observation.process.status,
        observation.process.stdout.text.as_deref(),
        observation.process.stderr.text.as_deref(),
    )
}

pub(super) fn rejected_by_runtime(observation: &ShapeObservation) -> bool {
    matches!(
        &observation.process.status,
        ProcessStatus::Exited { code: Some(code) } if *code != 0
    )
}

pub(super) fn output_mode_for_intent(intent: ProbeIntent) -> Option<OutputMode> {
    match intent {
        ProbeIntent::OutputJson | ProbeIntent::OutputJsonHelp => Some(OutputMode::Json),
        ProbeIntent::OutputYaml | ProbeIntent::OutputYamlHelp => Some(OutputMode::Yaml),
        ProbeIntent::OutputTable | ProbeIntent::OutputTableHelp => Some(OutputMode::Table),
        ProbeIntent::OutputPlain | ProbeIntent::OutputPlainHelp => Some(OutputMode::Plain),
        _ => None,
    }
}

pub(super) fn output_help_probe(intent: ProbeIntent) -> bool {
    matches!(
        intent,
        ProbeIntent::OutputJsonHelp
            | ProbeIntent::OutputYamlHelp
            | ProbeIntent::OutputTableHelp
            | ProbeIntent::OutputPlainHelp
    )
}

pub(super) fn output_probe_fragment(
    observation: &ShapeObservation,
    path: &[String],
) -> Vec<String> {
    let mut args = observation
        .process
        .argv
        .iter()
        .skip(1)
        .cloned()
        .collect::<Vec<_>>();
    if args.starts_with(path) {
        args.drain(..path.len());
    }
    if args.last().is_some_and(|arg| arg == "--help") {
        args.pop();
    }
    args
}

pub(super) fn argv_contains_fragment(argv_fragment: &[String], expected: &[String]) -> bool {
    if expected.is_empty() {
        return true;
    }
    argv_fragment
        .windows(expected.len())
        .any(|window| window == expected)
}

pub(super) fn relative_candidate_scope(
    current_path: &[String],
    candidates: &[layout::CandidateCommand],
) -> Vec<String> {
    if current_path.is_empty() {
        return Vec::new();
    }

    let relative_candidates = candidates
        .iter()
        .filter(|candidate| !candidate.absolute)
        .collect::<Vec<_>>();
    if relative_candidates.is_empty() {
        return current_path.to_vec();
    }

    for prefix_len in (0..current_path.len()).rev() {
        let suffix = &current_path[prefix_len..];
        if relative_candidates
            .iter()
            .any(|candidate| candidate.path.as_slice() == suffix)
        {
            return current_path[..prefix_len].to_vec();
        }
    }

    current_path.to_vec()
}

pub(super) fn absolutize_candidate_path(
    current_path: &[String],
    candidate: Vec<String>,
    absolute: bool,
) -> Vec<String> {
    if absolute || current_path.is_empty() || candidate.starts_with(current_path) {
        return candidate;
    }

    let mut full_path = current_path.to_vec();
    full_path.extend(candidate);
    full_path
}

pub(super) fn is_child_path(current_path: &[String], candidate_path: &[String]) -> bool {
    !current_path.is_empty()
        && candidate_path.len() > current_path.len()
        && candidate_path.starts_with(current_path)
}
