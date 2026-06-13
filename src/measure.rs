use std::collections::{BTreeSet, VecDeque};

use crate::cli::MeasureArgs;
use crate::error::Result;
use crate::evidence::{
    EvidenceKind, EvidenceWriter, ProbeIntent, ProbeScheduled, ProcessCompleted, ProcessStatus,
    RunFinished, RunStarted,
};
use crate::fingerprint::{TargetFingerprint, fingerprint_target};
use crate::process::{ProbeSpec, TargetProcess};
use crate::shape::{self, ShapeObservation};

pub async fn measure(args: MeasureArgs) -> Result<()> {
    let target = fingerprint_target(&args.target).await?;
    let mut evidence = EvidenceWriter::create(&args.out).await?;

    evidence
        .append(EvidenceKind::RunStarted(RunStarted {
            target: target.clone(),
            artifact_dir: args.out.clone(),
        }))
        .await?;

    let binary_name = target_binary_name(&target);
    let mut frontier = ProbeFrontier::new(args.max_depth);
    frontier.seed(bootstrap_probes(&target));
    let process = TargetProcess::new(
        target.resolved.clone(),
        args.timeout(),
        args.output_limit_bytes,
    );
    let mut observations = Vec::new();
    let mut probes_completed = 0_usize;

    while probes_completed < args.max_probes {
        let Some(probe) = frontier.next() else {
            break;
        };
        probes_completed += 1;

        let probe_id = format!("p_{:06}", probes_completed);
        evidence
            .append(EvidenceKind::ProbeScheduled(ProbeScheduled {
                probe_id: probe_id.clone(),
                argv: probe.argv(&target.resolved),
                path: probe.path.clone(),
                intent: probe.intent,
            }))
            .await?;

        let intent = probe.intent;
        let path = probe.path.clone();
        let outcome = process.run(&probe).await?;
        let completed = ProcessCompleted::from_outcome(probe_id, outcome);
        let event_id = evidence
            .append(EvidenceKind::ProcessCompleted(completed.clone()))
            .await?;

        frontier.learn_from(&probe, &completed, &binary_name);

        observations.push(ShapeObservation {
            evidence_id: event_id,
            intent,
            path,
            process: completed,
        });
    }

    evidence
        .append(EvidenceKind::RunFinished(RunFinished { probes_completed }))
        .await?;

    shape::write_shape(&args.out, target, &observations).await
}

#[derive(Debug)]
struct ProbeFrontier {
    queue: VecDeque<ProbeSpec>,
    scheduled_args: BTreeSet<Vec<String>>,
    max_depth: usize,
}

impl ProbeFrontier {
    fn new(max_depth: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            scheduled_args: BTreeSet::new(),
            max_depth,
        }
    }

    fn seed(&mut self, probes: impl IntoIterator<Item = ProbeSpec>) {
        for probe in probes {
            self.schedule(probe);
        }
    }

    fn schedule(&mut self, probe: ProbeSpec) -> bool {
        if !self.scheduled_args.insert(probe.args.clone()) {
            return false;
        }

        self.queue.push_back(probe);
        true
    }

    fn next(&mut self) -> Option<ProbeSpec> {
        self.queue.pop_front()
    }

    fn learn_from(&mut self, probe: &ProbeSpec, completed: &ProcessCompleted, binary_name: &str) {
        if !matches!(probe.intent, ProbeIntent::Help) || !is_successful_help(completed) {
            return;
        }

        let Some(text) = completed.stdout.text.as_deref() else {
            return;
        };

        for path in shape::discover_command_paths(text, binary_name, &probe.path) {
            if path.is_empty() || path.len() > self.max_depth {
                continue;
            }

            self.schedule(ProbeSpec::path_help(path.clone()));
            self.schedule(ProbeSpec::help_path(path));
        }
    }
}

fn is_successful_help(completed: &ProcessCompleted) -> bool {
    matches!(&completed.status, ProcessStatus::Exited { code: Some(0) })
        && completed
            .stdout
            .text
            .as_deref()
            .is_some_and(shape::is_help_like)
}

fn bootstrap_probes(target: &TargetFingerprint) -> Vec<ProbeSpec> {
    let target_name = target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target");
    let invalid_command = format!("__cliare_unknown_{target_name}_command__");
    let invalid_flag = format!("--__cliare_unknown_{target_name}_flag__");

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

fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::ProbeFrontier;
    use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
    use crate::process::{OutputCapture, ProbeSpec};

    #[test]
    fn frontier_schedules_nested_help_probes_from_generic_help() {
        let mut frontier = ProbeFrontier::new(2);
        let probe = ProbeSpec::path_help(vec!["flow".to_owned()]);
        let completed = completed_help(
            "Commands:\n  search  Search indexed flows\n\nOptions:\n  --help  Print help\n",
        );

        frontier.learn_from(&probe, &completed, "rote");

        let scheduled = std::iter::from_fn(|| frontier.next())
            .map(|probe| probe.args)
            .collect::<Vec<_>>();

        assert!(scheduled.contains(&vec![
            "flow".to_owned(),
            "search".to_owned(),
            "--help".to_owned()
        ]));
        assert!(scheduled.contains(&vec![
            "help".to_owned(),
            "flow".to_owned(),
            "search".to_owned()
        ]));
    }

    #[test]
    fn frontier_respects_depth_limit() {
        let mut frontier = ProbeFrontier::new(1);
        let probe = ProbeSpec::path_help(vec!["flow".to_owned()]);
        let completed = completed_help("Commands:\n  search  Search indexed flows\n");

        frontier.learn_from(&probe, &completed, "rote");

        assert!(frontier.next().is_none());
    }

    fn completed_help(stdout: &str) -> ProcessCompleted {
        ProcessCompleted {
            probe_id: "p_000001".to_owned(),
            argv: vec!["rote".to_owned(), "--help".to_owned()],
            status: ProcessStatus::Exited { code: Some(0) },
            duration_ms: 1,
            stdout: output(stdout),
            stderr: output(""),
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

    #[test]
    fn bootstrap_contains_only_generic_safe_probes() {
        let probes = super::bootstrap_probes(&crate::fingerprint::TargetFingerprint {
            requested: "tool".into(),
            resolved: "/tmp/tool".into(),
            binary_sha256: "abc".to_owned(),
            size_bytes: 1,
        });

        assert!(probes.iter().any(|probe| probe.args == ["--help"]));
        assert!(probes.iter().any(|probe| probe.args == ["help"]));
        assert!(
            probes
                .iter()
                .any(|probe| matches!(probe.intent, ProbeIntent::InvalidCommand))
        );
    }
}
