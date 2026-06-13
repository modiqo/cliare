use crate::cli::MeasureArgs;
use crate::error::Result;
use crate::evidence::{
    EvidenceKind, EvidenceWriter, ProbeIntent, ProbeScheduled, RunFinished, RunStarted,
};
use crate::fingerprint::{TargetFingerprint, fingerprint_target};
use crate::process::{ProbeSpec, TargetProcess};

pub async fn measure(args: MeasureArgs) -> Result<()> {
    let target = fingerprint_target(&args.target).await?;
    let mut evidence = EvidenceWriter::create(&args.out).await?;

    evidence
        .append(EvidenceKind::RunStarted(RunStarted {
            target: target.clone(),
            artifact_dir: args.out.clone(),
        }))
        .await?;

    let probes = bootstrap_probes(&target);
    let probe_count = probes.len();
    let process = TargetProcess::new(
        target.resolved.clone(),
        args.timeout(),
        args.output_limit_bytes,
    );

    for (index, probe) in probes.into_iter().enumerate() {
        let probe_id = format!("p_{:06}", index + 1);
        evidence
            .append(EvidenceKind::ProbeScheduled(ProbeScheduled {
                probe_id: probe_id.clone(),
                argv: probe.argv(&target.resolved),
                intent: probe.intent,
            }))
            .await?;

        let outcome = process.run(probe).await?;
        evidence
            .append(EvidenceKind::ProcessCompleted(
                crate::evidence::ProcessCompleted::from_outcome(probe_id, outcome),
            ))
            .await?;
    }

    evidence
        .append(EvidenceKind::RunFinished(RunFinished {
            probes_completed: probe_count,
        }))
        .await
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
