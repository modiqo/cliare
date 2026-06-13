use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::ci::{self, CiArtifactSummary};
use crate::claims::ClaimSet;
use crate::cli::MeasureArgs;
use crate::error::{CliareError, Result};
use crate::evidence::{
    EvidenceKind, EvidenceWriter, ProbeIntent, ProbeScheduled, ProcessCompleted, RunFinished,
    RunStarted,
};
use crate::fingerprint::{TargetFingerprint, fingerprint_target};
use crate::observation::ShapeObservation;
use crate::planner::{
    ConvergencePolicy, DeterministicPlanner, ProbePlanner, bootstrap_invalid_command_token,
    bootstrap_invalid_flag_token,
};
use crate::process::{ProbeSpec, TargetProcess};
use crate::sandbox::Sandbox;
use crate::score::{self, ScoreRunContext};
use crate::shape;

const MEASUREMENT_CACHE_SCHEMA_VERSION: &str = "cliare.measure-cache.v1";
const MEASUREMENT_ENGINE: &str = "cliare-measure-v0";

#[derive(Debug, Clone)]
pub struct MeasurementSummary {
    pub target: TargetFingerprint,
    pub probes_completed: usize,
    pub evidence_path: PathBuf,
    pub shape_path: PathBuf,
    pub scorecard_path: PathBuf,
    pub report_path: PathBuf,
    pub ci_summary_path: PathBuf,
    pub sarif_path: PathBuf,
    pub junit_path: PathBuf,
    pub sandbox_profile: String,
    pub sandbox_root: PathBuf,
    pub sandbox_home: PathBuf,
    pub sandbox_workdir: PathBuf,
    pub sandbox_env_policy: String,
    pub score_total: f64,
    pub score_measured_weight: f64,
    pub score_max_weight: f64,
    pub score_model: String,
    pub score_status: String,
    pub findings: usize,
    pub output_contracts_discovered: usize,
    pub machine_readable_output_contracts: usize,
    pub output_mode_probes_completed: usize,
    pub output_mode_parse_successes: usize,
    pub side_effect_files_created: usize,
    pub side_effect_files_modified: usize,
    pub side_effect_files_deleted: usize,
    pub side_effect_files_total: usize,
    pub side_effect_probe_count: usize,
    pub credential_like_side_effects: usize,
    pub observed_max_depth: usize,
    pub traversal_profile: String,
    pub max_depth: usize,
    pub max_probes: usize,
    pub min_expected_value: u16,
    pub frontier_remaining: usize,
    pub highest_pending_expected_value: Option<u16>,
    pub candidates_skipped_by_depth: usize,
    pub candidates_skipped_by_convergence: usize,
    pub probes_skipped_by_budget: usize,
    pub budget_exhausted: bool,
    pub traversal_stop_reason: String,
    pub traversal_complete: bool,
    pub cache_hit: bool,
}

impl MeasurementSummary {
    pub fn set_ci_artifacts(&mut self, artifacts: CiArtifactSummary) {
        self.ci_summary_path = artifacts.summary_path;
        self.sarif_path = artifacts.sarif_path;
        self.junit_path = artifacts.junit_path;
    }

    pub fn terminal_summary(&self) -> String {
        let lines = [
            "CLIARE measure complete".to_owned(),
            format!("target: {}", self.target.requested.display()),
            format!("resolved: {}", self.target.resolved.display()),
            format!(
                "score: {:.1}/100 ({}, measured {:.2}/{:.2}, model {})",
                self.score_total,
                self.score_status,
                self.score_measured_weight,
                self.score_max_weight,
                self.score_model
            ),
            format!("cache: {}", if self.cache_hit { "hit" } else { "miss" }),
            format!("probes: {}", self.probes_completed),
            format!("findings: {}", self.findings),
            "output contracts:".to_owned(),
            format!("  discovered: {}", self.output_contracts_discovered),
            format!(
                "  machine-readable: {}",
                self.machine_readable_output_contracts
            ),
            format!("  probes completed: {}", self.output_mode_probes_completed),
            format!("  parse successes: {}", self.output_mode_parse_successes),
            "side effects:".to_owned(),
            format!("  file changes: {}", self.side_effect_files_total),
            format!("  probes with changes: {}", self.side_effect_probe_count),
            format!("  created: {}", self.side_effect_files_created),
            format!("  modified: {}", self.side_effect_files_modified),
            format!("  deleted: {}", self.side_effect_files_deleted),
            format!(
                "  credential-like paths: {}",
                self.credential_like_side_effects
            ),
            "runtime isolation:".to_owned(),
            format!("  sandbox profile: {}", self.sandbox_profile),
            format!("  env policy: {}", self.sandbox_env_policy),
            format!("  sandbox root: {}", self.sandbox_root.display()),
            format!("  sandbox home: {}", self.sandbox_home.display()),
            format!("  sandbox workdir: {}", self.sandbox_workdir.display()),
            "coverage pressure:".to_owned(),
            format!("  profile: {}", self.traversal_profile),
            format!(
                "  depth: observed {} / budget {}",
                self.observed_max_depth, self.max_depth
            ),
            format!(
                "  probes: completed {} / budget {}",
                self.probes_completed, self.max_probes
            ),
            format!("  min expected value: {}", self.min_expected_value),
            format!("  frontier remaining: {}", self.frontier_remaining),
            format!(
                "  highest pending expected value: {}",
                self.highest_pending_expected_value
                    .map_or_else(|| "none".to_owned(), |value| value.to_string())
            ),
            format!("  skipped by depth: {}", self.candidates_skipped_by_depth),
            format!(
                "  skipped by convergence: {}",
                self.candidates_skipped_by_convergence
            ),
            format!(
                "  skipped by probe budget: {}",
                self.probes_skipped_by_budget
            ),
            format!("  budget exhausted: {}", self.budget_exhausted),
            format!("  stop reason: {}", self.traversal_stop_reason),
            format!("  traversal complete: {}", self.traversal_complete),
            "artifacts:".to_owned(),
            format!("  evidence: {}", self.evidence_path.display()),
            format!("  shape: {}", self.shape_path.display()),
            format!("  scorecard: {}", self.scorecard_path.display()),
            format!("  report: {}", self.report_path.display()),
            format!("  ci summary: {}", self.ci_summary_path.display()),
            format!("  sarif: {}", self.sarif_path.display()),
            format!("  junit: {}", self.junit_path.display()),
        ];

        format!("{}\n", lines.join("\n"))
    }
}

pub async fn measure(args: MeasureArgs) -> Result<MeasurementSummary> {
    let target = fingerprint_target(&args.target).await?;
    let max_depth = args.resolved_max_depth();
    let max_probes = args.resolved_max_probes();
    let min_expected_value = args.resolved_min_expected_value();
    let profile = ProbeProfile::from_args(
        &args,
        max_depth,
        max_probes,
        min_expected_value,
        crate::sandbox::SandboxProfile::Isolated.label(),
    );

    if !args.refresh
        && let Some(summary) = cached_summary(&args.out, &target, &profile).await?
    {
        return Ok(summary);
    }

    let sandbox = Sandbox::create(&args.out).await?;
    let mut evidence = EvidenceWriter::create(&args.out).await?;

    evidence
        .append(EvidenceKind::RunStarted(RunStarted {
            target: target.clone(),
            artifact_dir: args.out.clone(),
            sandbox: sandbox.metadata().clone(),
        }))
        .await?;

    let binary_name = target_binary_name(&target);
    let mut planner = DeterministicPlanner::with_policy(
        max_depth,
        ConvergencePolicy::new(min_expected_value),
        invalid_token_seed(&binary_name),
    );
    planner.seed(bootstrap_probes(&target));
    let process = TargetProcess::new(
        target.resolved.clone(),
        args.timeout(),
        args.output_limit_bytes,
        sandbox.execution(),
    );
    let mut observations = Vec::new();
    let mut probes_completed = 0_usize;

    while probes_completed < max_probes {
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
                sandbox: sandbox.probe_evidence(),
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
    let planner_stats = planner.stats();
    let run_context = ScoreRunContext {
        max_depth: planner_stats.max_depth,
        max_probes,
        min_expected_value: planner_stats.min_expected_value,
        traversal_profile: args.profile.label(),
        frontier_remaining: planner_stats.frontier_remaining,
        highest_pending_expected_value: planner_stats.highest_pending_expected_value,
        candidates_skipped_by_depth: planner_stats.candidates_skipped_by_depth,
        candidates_skipped_by_convergence: planner_stats.candidates_skipped_by_convergence,
        sandbox: score::SandboxScoreContext::from(sandbox.metadata()),
    };
    let score_artifacts =
        score::write_score_artifacts(&args.out, target.clone(), &observations, run_context).await?;
    let ci_artifacts = ci::write_ci_artifacts(&args.out, None).await?;

    let summary = MeasurementSummary {
        target,
        probes_completed,
        evidence_path: args.out.join("evidence.jsonl"),
        shape_path: args.out.join("shape.json"),
        scorecard_path: score_artifacts.scorecard_path,
        report_path: score_artifacts.report_path,
        ci_summary_path: ci_artifacts.summary_path,
        sarif_path: ci_artifacts.sarif_path,
        junit_path: ci_artifacts.junit_path,
        sandbox_profile: score_artifacts.sandbox_profile.to_owned(),
        sandbox_root: score_artifacts.sandbox_root,
        sandbox_home: score_artifacts.sandbox_home,
        sandbox_workdir: score_artifacts.sandbox_workdir,
        sandbox_env_policy: score_artifacts.sandbox_env_policy.to_owned(),
        score_total: score_artifacts.total,
        score_measured_weight: score_artifacts.measured_weight,
        score_max_weight: score_artifacts.max_weight,
        score_model: score_artifacts.model.to_owned(),
        score_status: score_artifacts.status.to_owned(),
        findings: score_artifacts.findings,
        output_contracts_discovered: score_artifacts.output_contracts_discovered,
        machine_readable_output_contracts: score_artifacts.machine_readable_output_contracts,
        output_mode_probes_completed: score_artifacts.output_mode_probes_completed,
        output_mode_parse_successes: score_artifacts.output_mode_parse_successes,
        side_effect_files_created: score_artifacts.side_effect_files_created,
        side_effect_files_modified: score_artifacts.side_effect_files_modified,
        side_effect_files_deleted: score_artifacts.side_effect_files_deleted,
        side_effect_files_total: score_artifacts.side_effect_files_total,
        side_effect_probe_count: score_artifacts.side_effect_probe_count,
        credential_like_side_effects: score_artifacts.credential_like_side_effects,
        observed_max_depth: score_artifacts.observed_max_depth,
        traversal_profile: score_artifacts.traversal_profile.to_owned(),
        max_depth: score_artifacts.max_depth,
        max_probes: score_artifacts.max_probes,
        min_expected_value: score_artifacts.min_expected_value,
        frontier_remaining: score_artifacts.frontier_remaining,
        highest_pending_expected_value: score_artifacts.highest_pending_expected_value,
        candidates_skipped_by_depth: score_artifacts.candidates_skipped_by_depth,
        candidates_skipped_by_convergence: score_artifacts.candidates_skipped_by_convergence,
        probes_skipped_by_budget: score_artifacts.probes_skipped_by_budget,
        budget_exhausted: score_artifacts.budget_exhausted,
        traversal_stop_reason: score_artifacts.traversal_stop_reason.to_owned(),
        traversal_complete: score_artifacts.traversal_complete,
        cache_hit: false,
    };
    write_cache_manifest(&args.out, &summary, profile).await?;

    Ok(summary)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct ProbeProfile {
    traversal_profile: crate::cli::TraversalProfile,
    sandbox_profile: String,
    timeout_ms: u64,
    output_limit_bytes: usize,
    max_depth: usize,
    max_probes: usize,
    min_expected_value: u16,
}

impl ProbeProfile {
    fn from_args(
        args: &MeasureArgs,
        max_depth: usize,
        max_probes: usize,
        min_expected_value: u16,
        sandbox_profile: &str,
    ) -> Self {
        Self {
            traversal_profile: args.profile,
            sandbox_profile: sandbox_profile.to_owned(),
            timeout_ms: args.timeout_ms,
            output_limit_bytes: args.output_limit_bytes,
            max_depth,
            max_probes,
            min_expected_value,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct MeasurementCacheManifest {
    schema_version: String,
    cliare_version: String,
    engine: String,
    target: TargetFingerprint,
    profile: ProbeProfile,
    summary: CachedMeasurementSummary,
}

#[derive(Debug, Deserialize, Serialize)]
struct CachedMeasurementSummary {
    probes_completed: usize,
    score_total: f64,
    score_measured_weight: f64,
    score_max_weight: f64,
    score_model: String,
    score_status: String,
    sandbox_profile: String,
    sandbox_root: PathBuf,
    sandbox_home: PathBuf,
    sandbox_workdir: PathBuf,
    sandbox_env_policy: String,
    findings: usize,
    output_contracts_discovered: usize,
    machine_readable_output_contracts: usize,
    output_mode_probes_completed: usize,
    output_mode_parse_successes: usize,
    side_effect_files_created: usize,
    side_effect_files_modified: usize,
    side_effect_files_deleted: usize,
    side_effect_files_total: usize,
    side_effect_probe_count: usize,
    credential_like_side_effects: usize,
    observed_max_depth: usize,
    traversal_profile: String,
    max_depth: usize,
    max_probes: usize,
    min_expected_value: u16,
    frontier_remaining: usize,
    highest_pending_expected_value: Option<u16>,
    candidates_skipped_by_depth: usize,
    candidates_skipped_by_convergence: usize,
    probes_skipped_by_budget: usize,
    budget_exhausted: bool,
    traversal_stop_reason: String,
    traversal_complete: bool,
}

async fn cached_summary(
    out_dir: &std::path::Path,
    target: &TargetFingerprint,
    profile: &ProbeProfile,
) -> Result<Option<MeasurementSummary>> {
    let path = out_dir.join("measure-cache.json");
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliareError::ReadMeasurementCache { path, source });
        }
    };
    let manifest: MeasurementCacheManifest =
        serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseMeasurementCache {
            path: path.clone(),
            source,
        })?;

    if !manifest.matches(target, profile) || !artifacts_exist(out_dir).await? {
        return Ok(None);
    }

    Ok(Some(manifest.into_summary(out_dir)))
}

impl MeasurementCacheManifest {
    fn matches(&self, target: &TargetFingerprint, profile: &ProbeProfile) -> bool {
        self.schema_version == MEASUREMENT_CACHE_SCHEMA_VERSION
            && self.cliare_version == env!("CARGO_PKG_VERSION")
            && self.engine == MEASUREMENT_ENGINE
            && &self.target == target
            && &self.profile == profile
    }

    fn into_summary(self, out_dir: &std::path::Path) -> MeasurementSummary {
        MeasurementSummary {
            target: self.target,
            probes_completed: self.summary.probes_completed,
            evidence_path: out_dir.join("evidence.jsonl"),
            shape_path: out_dir.join("shape.json"),
            scorecard_path: out_dir.join("scorecard.json"),
            report_path: out_dir.join("report.md"),
            ci_summary_path: out_dir.join("summary.md"),
            sarif_path: out_dir.join("findings.sarif"),
            junit_path: out_dir.join("junit.xml"),
            score_total: self.summary.score_total,
            score_measured_weight: self.summary.score_measured_weight,
            score_max_weight: self.summary.score_max_weight,
            score_model: self.summary.score_model,
            score_status: self.summary.score_status,
            sandbox_profile: self.summary.sandbox_profile,
            sandbox_root: self.summary.sandbox_root,
            sandbox_home: self.summary.sandbox_home,
            sandbox_workdir: self.summary.sandbox_workdir,
            sandbox_env_policy: self.summary.sandbox_env_policy,
            findings: self.summary.findings,
            output_contracts_discovered: self.summary.output_contracts_discovered,
            machine_readable_output_contracts: self.summary.machine_readable_output_contracts,
            output_mode_probes_completed: self.summary.output_mode_probes_completed,
            output_mode_parse_successes: self.summary.output_mode_parse_successes,
            side_effect_files_created: self.summary.side_effect_files_created,
            side_effect_files_modified: self.summary.side_effect_files_modified,
            side_effect_files_deleted: self.summary.side_effect_files_deleted,
            side_effect_files_total: self.summary.side_effect_files_total,
            side_effect_probe_count: self.summary.side_effect_probe_count,
            credential_like_side_effects: self.summary.credential_like_side_effects,
            observed_max_depth: self.summary.observed_max_depth,
            traversal_profile: self.summary.traversal_profile,
            max_depth: self.summary.max_depth,
            max_probes: self.summary.max_probes,
            min_expected_value: self.summary.min_expected_value,
            frontier_remaining: self.summary.frontier_remaining,
            highest_pending_expected_value: self.summary.highest_pending_expected_value,
            candidates_skipped_by_depth: self.summary.candidates_skipped_by_depth,
            candidates_skipped_by_convergence: self.summary.candidates_skipped_by_convergence,
            probes_skipped_by_budget: self.summary.probes_skipped_by_budget,
            budget_exhausted: self.summary.budget_exhausted,
            traversal_stop_reason: self.summary.traversal_stop_reason,
            traversal_complete: self.summary.traversal_complete,
            cache_hit: true,
        }
    }
}

async fn artifacts_exist(out_dir: &std::path::Path) -> Result<bool> {
    for name in [
        "evidence.jsonl",
        "shape.json",
        "scorecard.json",
        "report.md",
        "summary.md",
        "findings.sarif",
        "junit.xml",
    ] {
        let path = out_dir.join(name);
        match fs::metadata(&path).await {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Ok(false),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(source) => {
                return Err(CliareError::ReadMeasurementCache { path, source });
            }
        }
    }
    Ok(true)
}

async fn write_cache_manifest(
    out_dir: &std::path::Path,
    summary: &MeasurementSummary,
    profile: ProbeProfile,
) -> Result<()> {
    let path = out_dir.join("measure-cache.json");
    let manifest = MeasurementCacheManifest {
        schema_version: MEASUREMENT_CACHE_SCHEMA_VERSION.to_owned(),
        cliare_version: env!("CARGO_PKG_VERSION").to_owned(),
        engine: MEASUREMENT_ENGINE.to_owned(),
        target: summary.target.clone(),
        profile,
        summary: CachedMeasurementSummary {
            probes_completed: summary.probes_completed,
            score_total: summary.score_total,
            score_measured_weight: summary.score_measured_weight,
            score_max_weight: summary.score_max_weight,
            score_model: summary.score_model.to_owned(),
            score_status: summary.score_status.to_owned(),
            sandbox_profile: summary.sandbox_profile.to_owned(),
            sandbox_root: summary.sandbox_root.clone(),
            sandbox_home: summary.sandbox_home.clone(),
            sandbox_workdir: summary.sandbox_workdir.clone(),
            sandbox_env_policy: summary.sandbox_env_policy.to_owned(),
            findings: summary.findings,
            output_contracts_discovered: summary.output_contracts_discovered,
            machine_readable_output_contracts: summary.machine_readable_output_contracts,
            output_mode_probes_completed: summary.output_mode_probes_completed,
            output_mode_parse_successes: summary.output_mode_parse_successes,
            side_effect_files_created: summary.side_effect_files_created,
            side_effect_files_modified: summary.side_effect_files_modified,
            side_effect_files_deleted: summary.side_effect_files_deleted,
            side_effect_files_total: summary.side_effect_files_total,
            side_effect_probe_count: summary.side_effect_probe_count,
            credential_like_side_effects: summary.credential_like_side_effects,
            observed_max_depth: summary.observed_max_depth,
            traversal_profile: summary.traversal_profile.to_owned(),
            max_depth: summary.max_depth,
            max_probes: summary.max_probes,
            min_expected_value: summary.min_expected_value,
            frontier_remaining: summary.frontier_remaining,
            highest_pending_expected_value: summary.highest_pending_expected_value,
            candidates_skipped_by_depth: summary.candidates_skipped_by_depth,
            candidates_skipped_by_convergence: summary.candidates_skipped_by_convergence,
            probes_skipped_by_budget: summary.probes_skipped_by_budget,
            budget_exhausted: summary.budget_exhausted,
            traversal_stop_reason: summary.traversal_stop_reason.to_owned(),
            traversal_complete: summary.traversal_complete,
        },
    };
    let bytes =
        serde_json::to_vec_pretty(&manifest).map_err(CliareError::SerializeMeasurementCache)?;
    fs::write(&path, bytes)
        .await
        .map_err(|source| CliareError::WriteMeasurementCache { path, source })
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
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::cli::{MeasureArgs, TraversalProfile};
    use crate::evidence::ProbeIntent;
    use crate::fingerprint::TargetFingerprint;

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

    #[test]
    fn terminal_summary_lists_score_and_artifacts() {
        let summary = super::MeasurementSummary {
            target: TargetFingerprint {
                requested: "tool".into(),
                resolved: "/tmp/tool".into(),
                binary_sha256: "abc".to_owned(),
                size_bytes: 1,
            },
            probes_completed: 7,
            evidence_path: PathBuf::from(".cliare/evidence.jsonl"),
            shape_path: PathBuf::from(".cliare/shape.json"),
            scorecard_path: PathBuf::from(".cliare/scorecard.json"),
            report_path: PathBuf::from(".cliare/report.md"),
            ci_summary_path: PathBuf::from(".cliare/summary.md"),
            sarif_path: PathBuf::from(".cliare/findings.sarif"),
            junit_path: PathBuf::from(".cliare/junit.xml"),
            sandbox_profile: "isolated".to_owned(),
            sandbox_root: PathBuf::from(".cliare/sandbox"),
            sandbox_home: PathBuf::from(".cliare/sandbox/home"),
            sandbox_workdir: PathBuf::from(".cliare/sandbox/cwd"),
            sandbox_env_policy: "cleared_with_allowlist".to_owned(),
            score_total: 82.4,
            score_measured_weight: 0.9,
            score_max_weight: 1.0,
            score_model: "cliare-score-v0".to_owned(),
            score_status: "experimental partial".to_owned(),
            findings: 2,
            output_contracts_discovered: 1,
            machine_readable_output_contracts: 1,
            output_mode_probes_completed: 1,
            output_mode_parse_successes: 1,
            side_effect_files_created: 0,
            side_effect_files_modified: 0,
            side_effect_files_deleted: 0,
            side_effect_files_total: 0,
            side_effect_probe_count: 0,
            credential_like_side_effects: 0,
            observed_max_depth: 1,
            traversal_profile: "standard".to_owned(),
            max_depth: 5,
            max_probes: 256,
            min_expected_value: 150,
            frontier_remaining: 0,
            highest_pending_expected_value: None,
            candidates_skipped_by_depth: 0,
            candidates_skipped_by_convergence: 0,
            probes_skipped_by_budget: 0,
            budget_exhausted: false,
            traversal_stop_reason: "converged".to_owned(),
            traversal_complete: true,
            cache_hit: false,
        };

        let text = summary.terminal_summary();

        assert!(text.contains("CLIARE measure complete"));
        assert!(text.contains("score: 82.4/100"));
        assert!(text.contains("cache: miss"));
        assert!(text.contains("output contracts:"));
        assert!(text.contains("machine-readable: 1"));
        assert!(text.contains("side effects:"));
        assert!(text.contains("file changes: 0"));
        assert!(text.contains("sandbox profile: isolated"));
        assert!(text.contains("env policy: cleared_with_allowlist"));
        assert!(text.contains("depth: observed 1 / budget 5"));
        assert!(text.contains("min expected value: 150"));
        assert!(text.contains("stop reason: converged"));
        assert!(text.contains("  scorecard: .cliare/scorecard.json"));
        assert!(text.contains("  report: .cliare/report.md"));
        assert!(text.contains("  ci summary: .cliare/summary.md"));
        assert!(text.contains("  sarif: .cliare/findings.sarif"));
        assert!(text.contains("  junit: .cliare/junit.xml"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn measure_runs_probes_inside_isolated_sandbox() {
        use std::os::unix::fs::PermissionsExt;

        let root = unique_test_dir("sandbox-measure");
        let bin_dir = root.join("bin");
        let out_dir = root.join("out");
        fs::create_dir_all(&bin_dir).expect("creates fixture bin dir");

        let target = bin_dir.join("writes-home");
        fs::write(
            &target,
            r#"#!/bin/sh
case "$HOME" in
  */sandbox/home) ;;
  *) echo "unexpected HOME: $HOME" >&2; exit 99 ;;
esac
case "$PWD" in
  */sandbox/cwd) ;;
  *) echo "unexpected PWD: $PWD" >&2; exit 98 ;;
esac
printf ok > "$HOME/home-marker"
printf ok > "$PWD/cwd-marker"
cat <<'EOF'
Usage: writes-home [COMMAND]

Commands:
  alpha  Sample command

Options:
  --help  Print help
EOF
"#,
        )
        .expect("writes fixture cli");
        let mut permissions = fs::metadata(&target)
            .expect("reads fixture metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&target, permissions).expect("marks fixture executable");

        let summary = super::measure(MeasureArgs {
            target,
            out: out_dir.clone(),
            timeout_ms: 1_000,
            output_limit_bytes: 16 * 1024,
            profile: TraversalProfile::Quick,
            max_depth: Some(1),
            max_probes: Some(1),
            min_expected_value: Some(1),
            refresh: true,
        })
        .await
        .expect("measurement succeeds");

        assert_eq!(summary.sandbox_profile, "isolated");
        assert_eq!(summary.sandbox_env_policy, "cleared_with_allowlist");
        assert!(out_dir.join("sandbox/home/home-marker").is_file());
        assert!(out_dir.join("sandbox/cwd/cwd-marker").is_file());
        assert!(!root.join("home-marker").exists());

        let _ = fs::remove_dir_all(root);
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cliare-{name}-{}-{nonce}", std::process::id()))
    }
}
