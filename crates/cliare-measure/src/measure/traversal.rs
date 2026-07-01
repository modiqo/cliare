use tokio::task::JoinHandle;

use crate::claims::ClaimSet;
use crate::error::{CliareError, Result};
use crate::evidence::{EvidenceKind, EvidenceWriter, ProbeScheduled, ProcessCompleted};
use crate::fingerprint::TargetFingerprint;
use crate::observation::ShapeObservation;
use crate::planner::{DeterministicPlanner, ProbePlanner};
use crate::process::{ProbeSpec, TargetProcess};
use crate::sandbox::Sandbox;

use super::checkpoint::{CheckpointObservation, CheckpointWriter, TraversalResume};
use super::progress::{ProgressCounters, ProgressLog};

#[derive(Debug)]
pub(super) struct TraversalRun {
    pub(super) observations: Vec<ShapeObservation>,
    pub(super) probes_scheduled: usize,
    pub(super) probes_completed: usize,
    pub(super) probes_cancelled: usize,
    pub(super) rounds: usize,
}

#[derive(Debug)]
struct ScheduledProbe {
    pub(super) probe_id: String,
    pub(super) probe: ProbeSpec,
    pub(super) handle: JoinHandle<Result<crate::process::ProbeOutcome>>,
}

pub(super) struct TraversalContext<'a> {
    pub(super) target: &'a TargetFingerprint,
    pub(super) sandbox: &'a Sandbox,
    pub(super) process: &'a TargetProcess,
    pub(super) evidence: &'a mut EvidenceWriter,
    pub(super) progress: &'a mut ProgressLog,
    pub(super) planner: &'a mut DeterministicPlanner,
    pub(super) binary_name: &'a str,
    pub(super) checkpoint: CheckpointWriter,
    pub(super) resume: Option<TraversalResume>,
    pub(super) limits: TraversalLimits,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TraversalLimits {
    pub(super) max_probes: usize,
    pub(super) concurrency_limit: usize,
}

pub(super) async fn run_traversal(context: TraversalContext<'_>) -> Result<TraversalRun> {
    let resume = context.resume.unwrap_or_default();
    let mut checkpoint_completed = resume.completed;
    let mut observations: Vec<ShapeObservation> = checkpoint_completed
        .iter()
        .map(|entry| entry.observation.clone())
        .collect();
    let mut probes_scheduled = resume.probes_scheduled;
    let mut probes_completed = resume.probes_completed;
    let mut rounds = resume.rounds;

    loop {
        let mut round = Vec::new();
        while round.len() < context.limits.concurrency_limit
            && probes_scheduled < context.limits.max_probes
        {
            let Some(probe) = context.planner.next() else {
                break;
            };
            probes_scheduled += 1;
            let probe_id = format!("p_{:06}", probes_scheduled);
            let execution = context.sandbox.execution_for_probe(&probe_id).await?;

            context
                .evidence
                .append(EvidenceKind::ProbeScheduled(ProbeScheduled {
                    probe_id: probe_id.clone(),
                    argv: probe.argv(&context.target.resolved),
                    path: probe.path.clone(),
                    intent: probe.intent,
                    sandbox: context.sandbox.probe_evidence_for(&execution),
                }))
                .await?;
            context
                .progress
                .scheduled(&probe_id, &probe, probes_scheduled, probes_completed)
                .await?;

            let process = context.process.clone();
            let task_probe = probe.clone();
            let handle = tokio::spawn(async move { process.run(&task_probe, execution).await });
            round.push(ScheduledProbe {
                probe_id,
                probe,
                handle,
            });
        }

        if round.is_empty() {
            break;
        }
        rounds += 1;
        context
            .progress
            .round_started(rounds, round.len(), probes_scheduled, probes_completed)
            .await?;

        let mut round_error = None;
        for scheduled in round {
            let outcome = match scheduled.handle.await {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(error)) => {
                    round_error.get_or_insert(error);
                    continue;
                }
                Err(error) => {
                    round_error.get_or_insert(CliareError::Join(error));
                    continue;
                }
            };
            probes_completed += 1;
            let probe_id = scheduled.probe_id.clone();
            let probe = scheduled.probe.clone();
            let completed = ProcessCompleted::from_outcome(probe_id.clone(), outcome);
            let event_id = context
                .evidence
                .append(EvidenceKind::ProcessCompleted(completed.clone()))
                .await?;

            let observation = ShapeObservation {
                evidence_id: event_id,
                intent: probe.intent,
                path: probe.path.clone(),
                process: completed.clone(),
            };
            observations.push(observation.clone());
            checkpoint_completed.push(CheckpointObservation {
                probe: probe.clone(),
                observation,
            });
            context
                .checkpoint
                .write(
                    context.evidence.next_event_id(),
                    &checkpoint_completed,
                    probes_scheduled,
                    probes_completed,
                    rounds,
                )
                .await?;

            let claims = ClaimSet::from_observations(context.binary_name, &observations);
            context.planner.extend_from_claims(&claims);
            context
                .progress
                .completed(
                    &probe_id,
                    &probe,
                    &completed,
                    ProgressCounters {
                        probes_scheduled,
                        probes_completed,
                        round: rounds,
                    },
                    context.planner.stats(),
                )
                .await?;
        }

        if let Some(error) = round_error {
            let _ = context.progress.failed(probes_completed, &error).await;
            return Err(error);
        }
    }

    Ok(TraversalRun {
        observations,
        probes_scheduled,
        probes_completed,
        probes_cancelled: probes_scheduled.saturating_sub(probes_completed),
        rounds,
    })
}
