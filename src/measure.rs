use crate::claims::ClaimSet;
use crate::cli::MeasureArgs;
use crate::error::Result;
use crate::evidence::{
    EvidenceKind, EvidenceWriter, ProbeIntent, ProbeScheduled, ProcessCompleted, RunFinished,
    RunStarted,
};
use crate::fingerprint::{TargetFingerprint, fingerprint_target};
use crate::observation::ShapeObservation;
use crate::planner::{
    DeterministicPlanner, ProbePlanner, bootstrap_invalid_command_token,
    bootstrap_invalid_flag_token,
};
use crate::process::{ProbeSpec, TargetProcess};
use crate::score;
use crate::shape;

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
    let mut planner = DeterministicPlanner::new(args.max_depth, invalid_token_seed(&binary_name));
    planner.seed(bootstrap_probes(&target));
    let process = TargetProcess::new(
        target.resolved.clone(),
        args.timeout(),
        args.output_limit_bytes,
    );
    let mut observations = Vec::new();
    let mut probes_completed = 0_usize;

    while probes_completed < args.max_probes {
        let Some(probe) = planner.next() else {
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

        observations.push(ShapeObservation {
            evidence_id: event_id,
            intent,
            path,
            process: completed,
        });

        let claims = ClaimSet::from_observations(&binary_name, &observations);
        planner.extend_from_claims(&claims);
    }

    evidence
        .append(EvidenceKind::RunFinished(RunFinished { probes_completed }))
        .await?;

    shape::write_shape(&args.out, target.clone(), &observations).await?;
    score::write_score_artifacts(&args.out, target, &observations).await
}

fn bootstrap_probes(target: &TargetFingerprint) -> Vec<ProbeSpec> {
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

fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}

fn invalid_token_seed(binary_name: &str) -> String {
    binary_name.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use crate::evidence::ProbeIntent;

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

    #[test]
    fn invalid_token_seed_is_shell_token_friendly() {
        assert_eq!(super::invalid_token_seed("my-tool"), "my_tool");
    }
}
