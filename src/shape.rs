use std::path::Path;

use serde::Serialize;
use tokio::fs;

use crate::claims::{ClaimSet, CommandClaim, FlagClaim};
use crate::error::{CliareError, Result};
use crate::fingerprint::TargetFingerprint;
use crate::observation::ShapeObservation;

const SCHEMA_VERSION: &str = "cliare.command-shape.v1";
const INFERENCE_MODEL: &str = "cliare-generic-claims-v0";

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
    ArgumentArityUnknown,
    InvalidChildDiagnosticsUnknown,
    InvalidFlagDiagnosticsUnknown,
}

#[derive(Debug, Serialize)]
pub struct InferenceModel {
    name: &'static str,
    source: &'static str,
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
    let binary_name = target_binary_name(&target);
    let claims = ClaimSet::from_observations(&binary_name, observations);
    let commands = claims
        .commands()
        .map(|command| command_candidate(&binary_name, command))
        .collect::<Vec<_>>();
    let flags = claims.flags().map(flag_candidate).collect::<Vec<_>>();
    let gaps = gap_items(claims.commands());

    CommandShape {
        schema_version: SCHEMA_VERSION,
        target,
        commands,
        flags,
        gaps,
        model: InferenceModel {
            name: INFERENCE_MODEL,
            source: "generic claim store with layout evidence, runtime confirmation, and diagnostic probes",
        },
    }
}

fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}

fn command_candidate(binary_name: &str, command: &CommandClaim) -> CommandCandidate {
    let path = command.path().to_vec();
    let mut argv = Vec::with_capacity(path.len() + 1);
    argv.push(binary_name.to_owned());
    argv.extend(path.iter().cloned());

    CommandCandidate {
        id: command_id(binary_name, &path),
        path,
        argv,
        summary: command.summary().map(str::to_owned),
        confidence: command.confidence(),
        runtime_confirmed: command.runtime_confirmed(),
        evidence: command.evidence().to_vec(),
    }
}

fn flag_candidate(flag: &FlagClaim) -> FlagCandidate {
    FlagCandidate {
        command_path: flag.command_path().to_vec(),
        name: flag.name().to_owned(),
        short: flag.short().map(str::to_owned),
        summary: flag.summary().map(str::to_owned),
        confidence: flag.confidence(),
        evidence: flag.evidence().to_vec(),
    }
}

fn gap_items<'a>(commands: impl Iterator<Item = &'a CommandClaim>) -> Vec<Gap> {
    let mut gaps = Vec::new();

    for command in commands {
        if command.confidence() < 0.80 {
            gaps.push(Gap {
                kind: GapKind::ExistenceUnconfirmed,
                command_path: command.path().to_vec(),
                reason: "candidate has not accumulated enough confirming runtime evidence"
                    .to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.help_unavailable() {
            gaps.push(Gap {
                kind: GapKind::HelpUnavailable,
                command_path: command.path().to_vec(),
                reason: "safe help probe did not produce help-like output".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.runtime_confirmed() {
            gaps.push(Gap {
                kind: GapKind::FlagsUnknown,
                command_path: command.path().to_vec(),
                reason: "flag arity and value domains are not confirmed yet".to_owned(),
                evidence: command.evidence().to_vec(),
            });
            gaps.push(Gap {
                kind: GapKind::ArgumentArityUnknown,
                command_path: command.path().to_vec(),
                reason: "positional argument arity is not confirmed yet".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.runtime_confirmed()
            && command.has_child_candidates()
            && !command.invalid_child_rejected()
        {
            gaps.push(Gap {
                kind: GapKind::InvalidChildDiagnosticsUnknown,
                command_path: command.path().to_vec(),
                reason: "safe invalid-child probe has not observed command diagnostics".to_owned(),
                evidence: command.evidence().to_vec(),
            });
        }
        if command.runtime_confirmed() && !command.invalid_flag_rejected() {
            gaps.push(Gap {
                kind: GapKind::InvalidFlagDiagnosticsUnknown,
                command_path: command.path().to_vec(),
                reason: "safe invalid-flag probe has not observed flag diagnostics".to_owned(),
                evidence: command.evidence().to_vec(),
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
    use super::infer_shape;
    use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
    use crate::fingerprint::TargetFingerprint;
    use crate::observation::ShapeObservation;
    use crate::process::OutputCapture;

    #[test]
    fn generic_layout_candidates_are_low_confidence_until_confirmed() {
        let target = target();
        let root = observation(
            "e_000003",
            ProbeIntent::Help,
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
            ProbeIntent::Help,
            vec![],
            "Commands:\n  measure  Run probes\n",
            Some(0),
        );
        let measure_help = observation(
            "e_000005",
            ProbeIntent::Help,
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
    fn shape_keeps_nested_candidates_from_child_help() {
        let target = target();
        let flow_help = observation(
            "e_000003",
            ProbeIntent::Help,
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

    #[test]
    fn diagnostic_probes_close_diagnostic_gaps() {
        let target = target();
        let observations = vec![
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["measure".to_owned()],
                "Usage: cliare measure <TARGET>\n\nCommands:\n  nested  Nested command\n\nOptions:\n  --out <DIR>  Output directory\n",
                Some(0),
            ),
            observation(
                "e_000007",
                ProbeIntent::InvalidChild,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
            observation(
                "e_000009",
                ProbeIntent::InvalidFlag,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
        ];

        let shape = infer_shape(target, &observations);
        let measure = shape
            .commands
            .iter()
            .find(|command| command.path == ["measure"])
            .expect("measure command exists");

        assert!(measure.runtime_confirmed);
        assert!(!shape.gaps.iter().any(|gap| {
            gap.command_path == ["measure"]
                && matches!(
                    gap.kind,
                    super::GapKind::InvalidChildDiagnosticsUnknown
                        | super::GapKind::InvalidFlagDiagnosticsUnknown
                )
        }));
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
        intent: ProbeIntent,
        path: Vec<String>,
        stdout: &str,
        exit_code: Option<i32>,
    ) -> ShapeObservation {
        ShapeObservation {
            evidence_id: evidence_id.to_owned(),
            intent,
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
