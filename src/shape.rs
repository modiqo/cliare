use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use tokio::fs;

use crate::belief::Belief;
use crate::error::{CliareError, Result};
use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
use crate::fingerprint::TargetFingerprint;
use crate::layout;

const SCHEMA_VERSION: &str = "cliare.command-shape.v1";
const INFERENCE_MODEL: &str = "cliare-generic-layout-v0";

#[derive(Debug)]
pub struct ShapeObservation {
    pub evidence_id: String,
    pub intent: ProbeIntent,
    pub path: Vec<String>,
    pub process: ProcessCompleted,
}

#[derive(Debug, Serialize)]
pub struct CommandShape {
    schema_version: &'static str,
    target: TargetFingerprint,
    commands: Vec<CommandCandidate>,
    flags: Vec<FlagCandidate>,
    gaps: Vec<Gap>,
    model: InferenceModel,
}

#[derive(Debug, Serialize)]
pub struct CommandCandidate {
    id: String,
    path: Vec<String>,
    argv: Vec<String>,
    summary: Option<String>,
    confidence: f64,
    runtime_confirmed: bool,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FlagCandidate {
    command_path: Vec<String>,
    name: String,
    short: Option<String>,
    summary: Option<String>,
    confidence: f64,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Gap {
    kind: GapKind,
    command_path: Vec<String>,
    reason: String,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    ExistenceUnconfirmed,
    HelpUnavailable,
    FlagsUnknown,
}

#[derive(Debug, Serialize)]
pub struct InferenceModel {
    name: &'static str,
    source: &'static str,
}

#[derive(Debug)]
struct CommandAccumulator {
    path: Vec<String>,
    summary: Option<String>,
    belief: Belief,
    runtime_confirmed: bool,
    help_unavailable: bool,
    evidence: Vec<String>,
}

impl CommandAccumulator {
    fn new(path: Vec<String>) -> Self {
        Self {
            path,
            summary: None,
            belief: Belief::with_prior(0.08),
            runtime_confirmed: false,
            help_unavailable: false,
            evidence: Vec::new(),
        }
    }

    fn apply_layout_candidate(
        &mut self,
        summary: Option<String>,
        evidence_id: &str,
        evidence_detail: &str,
    ) {
        self.belief.update(1.0);
        if self.summary.is_none() {
            self.summary = summary;
        }
        self.evidence
            .push(format!("{evidence_id}:{evidence_detail}"));
    }

    fn apply_runtime_confirmation(&mut self, evidence_id: &str, help_like: bool) {
        if help_like {
            self.belief.update(4.0);
            self.runtime_confirmed = true;
        } else {
            self.belief.update(-2.0);
            self.help_unavailable = true;
        }
        self.evidence.push(evidence_id.to_owned());
    }
}

#[derive(Debug)]
struct FlagAccumulator {
    command_path: Vec<String>,
    name: String,
    short: Option<String>,
    summary: Option<String>,
    belief: Belief,
    evidence: Vec<String>,
}

impl FlagAccumulator {
    fn new(command_path: Vec<String>, name: String) -> Self {
        Self {
            command_path,
            name,
            short: None,
            summary: None,
            belief: Belief::with_prior(0.12),
            evidence: Vec::new(),
        }
    }

    fn apply_layout_candidate(
        &mut self,
        short: Option<String>,
        summary: Option<String>,
        evidence_id: &str,
        evidence_detail: &str,
    ) {
        self.belief.update(1.0);
        if self.short.is_none() {
            self.short = short;
        }
        if self.summary.is_none() {
            self.summary = summary;
        }
        self.evidence
            .push(format!("{evidence_id}:{evidence_detail}"));
    }
}

pub async fn write_shape(
    out_dir: &Path,
    target: TargetFingerprint,
    observations: &[ShapeObservation],
) -> Result<()> {
    let shape = infer_shape(target, observations);
    let path = out_dir.join("shape.json");
    let bytes = serde_json::to_vec_pretty(&shape).map_err(CliareError::SerializeShape)?;
    fs::write(&path, bytes)
        .await
        .map_err(|source| CliareError::WriteShape { path, source })
}

pub fn infer_shape(target: TargetFingerprint, observations: &[ShapeObservation]) -> CommandShape {
    let binary_name = target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target");
    let mut commands = BTreeMap::<Vec<String>, CommandAccumulator>::new();
    let mut flags = BTreeMap::<(Vec<String>, String), FlagAccumulator>::new();

    for observation in observations {
        if !matches!(observation.intent, ProbeIntent::Help) {
            continue;
        }

        let help_like = successful_help_like(observation);
        if !observation.path.is_empty() {
            commands
                .entry(observation.path.clone())
                .or_insert_with(|| CommandAccumulator::new(observation.path.clone()))
                .apply_runtime_confirmation(&observation.evidence_id, help_like);
        }

        let Some(text) = observation.process.stdout.text.as_deref() else {
            continue;
        };
        if !help_like && !observation.path.is_empty() {
            continue;
        }

        for candidate in layout::command_candidates(text, binary_name) {
            let full_path = absolutize_candidate_path(&observation.path, candidate.path);
            if full_path == observation.path {
                continue;
            }
            commands
                .entry(full_path.clone())
                .or_insert_with(|| CommandAccumulator::new(full_path))
                .apply_layout_candidate(
                    candidate.summary,
                    &observation.evidence_id,
                    &candidate.evidence_detail,
                );
        }

        for candidate in layout::flag_candidates(text) {
            let key = (observation.path.clone(), candidate.name.clone());
            flags
                .entry(key)
                .or_insert_with(|| {
                    FlagAccumulator::new(observation.path.clone(), candidate.name.clone())
                })
                .apply_layout_candidate(
                    candidate.short,
                    candidate.summary,
                    &observation.evidence_id,
                    &candidate.evidence_detail,
                );
        }
    }

    let command_items = commands
        .values()
        .map(|command| command_candidate(binary_name, command))
        .collect::<Vec<_>>();
    let flag_items = flags.values().map(flag_candidate).collect::<Vec<_>>();
    let gaps = gap_items(commands.values());

    CommandShape {
        schema_version: SCHEMA_VERSION,
        target,
        commands: command_items,
        flags: flag_items,
        gaps,
        model: InferenceModel {
            name: INFERENCE_MODEL,
            source: "generic layout candidates plus runtime confirmation",
        },
    }
}

pub fn discover_command_paths(
    text: &str,
    binary_name: &str,
    current_path: &[String],
) -> Vec<Vec<String>> {
    layout::command_candidates(text, binary_name)
        .into_iter()
        .map(|candidate| absolutize_candidate_path(current_path, candidate.path))
        .collect()
}

pub fn is_help_like(text: &str) -> bool {
    layout::is_help_like(text)
}

fn successful_help_like(observation: &ShapeObservation) -> bool {
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

fn absolutize_candidate_path(current_path: &[String], candidate: Vec<String>) -> Vec<String> {
    if current_path.is_empty() || candidate.starts_with(current_path) {
        return candidate;
    }

    let mut full_path = current_path.to_vec();
    full_path.extend(candidate);
    full_path
}

fn command_candidate(binary_name: &str, command: &CommandAccumulator) -> CommandCandidate {
    let mut argv = Vec::with_capacity(command.path.len() + 1);
    argv.push(binary_name.to_owned());
    argv.extend(command.path.iter().cloned());

    CommandCandidate {
        id: command_id(binary_name, &command.path),
        path: command.path.clone(),
        argv,
        summary: command.summary.clone(),
        confidence: command.belief.probability(),
        runtime_confirmed: command.runtime_confirmed,
        evidence: command.evidence.clone(),
    }
}

fn flag_candidate(flag: &FlagAccumulator) -> FlagCandidate {
    FlagCandidate {
        command_path: flag.command_path.clone(),
        name: flag.name.clone(),
        short: flag.short.clone(),
        summary: flag.summary.clone(),
        confidence: flag.belief.probability(),
        evidence: flag.evidence.clone(),
    }
}

fn gap_items<'a>(commands: impl Iterator<Item = &'a CommandAccumulator>) -> Vec<Gap> {
    let mut gaps = Vec::new();

    for command in commands {
        if command.belief.probability() < 0.80 {
            gaps.push(Gap {
                kind: GapKind::ExistenceUnconfirmed,
                command_path: command.path.clone(),
                reason: "candidate has not accumulated enough confirming runtime evidence"
                    .to_owned(),
                evidence: command.evidence.clone(),
            });
        }
        if command.help_unavailable {
            gaps.push(Gap {
                kind: GapKind::HelpUnavailable,
                command_path: command.path.clone(),
                reason: "safe help probe did not produce help-like output".to_owned(),
                evidence: command.evidence.clone(),
            });
        }
        if command.runtime_confirmed {
            gaps.push(Gap {
                kind: GapKind::FlagsUnknown,
                command_path: command.path.clone(),
                reason: "flag arity and value domains are not confirmed yet".to_owned(),
                evidence: command.evidence.clone(),
            });
        }
    }

    gaps
}

fn command_id(binary_name: &str, path: &[String]) -> String {
    let mut id = binary_name.to_owned();
    for segment in path {
        id.push('.');
        id.push_str(segment);
    }
    id
}

#[cfg(test)]
mod tests {
    use super::{ShapeObservation, discover_command_paths, infer_shape};
    use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
    use crate::fingerprint::TargetFingerprint;
    use crate::process::OutputCapture;

    #[test]
    fn generic_layout_candidates_are_low_confidence_until_confirmed() {
        let target = target();
        let root = observation(
            "e_000003",
            vec![],
            "Commands:\n  measure  Run probes\n\nOptions:\n  -h, --help     Print help\n",
            Some(0),
        );

        let shape = infer_shape(target, &[root]);

        let measure = shape
            .commands
            .iter()
            .find(|command| command.path == ["measure"])
            .expect("measure candidate exists");
        assert!(!measure.runtime_confirmed);
        assert!(measure.confidence < 0.80);
        assert!(shape.flags.iter().any(|flag| flag.name == "--help"));
        assert!(shape.gaps.iter().any(|gap| gap.command_path == ["measure"]));
    }

    #[test]
    fn runtime_help_confirmation_raises_command_confidence() {
        let target = target();
        let root = observation(
            "e_000003",
            vec![],
            "Commands:\n  measure  Run probes\n",
            Some(0),
        );
        let measure_help = observation(
            "e_000005",
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
            Some(0),
        );

        let shape = infer_shape(target, &[root, measure_help]);

        let measure = shape
            .commands
            .iter()
            .find(|command| command.path == ["measure"])
            .expect("measure candidate exists");
        assert!(measure.runtime_confirmed);
        assert!(measure.confidence > 0.90);
    }

    #[test]
    fn discovery_absolutizes_nested_candidates() {
        let paths = discover_command_paths(
            "Commands:\n  search  Search flows\n",
            "cliare",
            &["flow".to_owned()],
        );

        assert_eq!(paths, vec![vec!["flow".to_owned(), "search".to_owned()]]);
    }

    #[test]
    fn shape_keeps_nested_candidates_from_child_help() {
        let target = target();
        let flow_help = observation(
            "e_000003",
            vec!["flow".to_owned()],
            "Commands:\n  search  Search flows\n",
            Some(0),
        );

        let shape = infer_shape(target, &[flow_help]);

        assert!(
            shape
                .commands
                .iter()
                .any(|command| command.path == ["flow", "search"])
        );
    }

    fn target() -> TargetFingerprint {
        TargetFingerprint {
            requested: "cliare".into(),
            resolved: "/tmp/cliare".into(),
            binary_sha256: "abc".to_owned(),
            size_bytes: 1,
        }
    }

    fn observation(
        evidence_id: &str,
        path: Vec<String>,
        stdout: &str,
        exit_code: Option<i32>,
    ) -> ShapeObservation {
        ShapeObservation {
            evidence_id: evidence_id.to_owned(),
            intent: ProbeIntent::Help,
            path,
            process: ProcessCompleted {
                probe_id: "p_000001".to_owned(),
                argv: vec!["cliare".to_owned(), "--help".to_owned()],
                status: ProcessStatus::Exited { code: exit_code },
                duration_ms: 1,
                stdout: output(stdout),
                stderr: output(""),
            },
        }
    }

    fn output(text: &str) -> OutputCapture {
        OutputCapture {
            sha256: "unused".to_owned(),
            bytes: text.len(),
            retained_bytes: text.len(),
            truncated: false,
            text: Some(text.to_owned()),
        }
    }
}
