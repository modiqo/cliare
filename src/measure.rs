use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;

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
    DeterministicPlanner, ProbePlanner, bootstrap_invalid_command_token,
    bootstrap_invalid_flag_token,
};
use crate::process::{ProbeSpec, TargetProcess};
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
    pub score_total: f64,
    pub score_measured_weight: f64,
    pub score_max_weight: f64,
    pub score_model: String,
    pub score_status: String,
    pub findings: usize,
    pub observed_max_depth: usize,
    pub max_depth: usize,
    pub max_probes: usize,
    pub frontier_remaining: usize,
    pub candidates_skipped_by_depth: usize,
    pub probes_skipped_by_budget: usize,
    pub budget_exhausted: bool,
    pub cache_hit: bool,
}

impl MeasurementSummary {
    pub fn terminal_summary(&self) -> String {
        let lines = [
            "CLIARE measure complete".to_owned(),
            format!("target: {}", self.target.requested.display()),
            format!("resolved: {}", self.target.resolved.display()),
            format!(
                "score: {:.1}/100 ({}, measured {:.1}/{:.1}, model {})",
                self.score_total,
                self.score_status,
                self.score_measured_weight,
                self.score_max_weight,
                self.score_model
            ),
            format!("cache: {}", if self.cache_hit { "hit" } else { "miss" }),
            format!("probes: {}", self.probes_completed),
            format!("findings: {}", self.findings),
            "coverage pressure:".to_owned(),
            format!(
                "  depth: observed {} / budget {}",
                self.observed_max_depth, self.max_depth
            ),
            format!(
                "  probes: completed {} / budget {}",
                self.probes_completed, self.max_probes
            ),
            format!("  frontier remaining: {}", self.frontier_remaining),
            format!("  skipped by depth: {}", self.candidates_skipped_by_depth),
            format!(
                "  skipped by probe budget: {}",
                self.probes_skipped_by_budget
            ),
            format!("  budget exhausted: {}", self.budget_exhausted),
            "artifacts:".to_owned(),
            format!("  evidence: {}", self.evidence_path.display()),
            format!("  shape: {}", self.shape_path.display()),
            format!("  scorecard: {}", self.scorecard_path.display()),
            format!("  report: {}", self.report_path.display()),
        ];

        format!("{}\n", lines.join("\n"))
    }
}

pub async fn measure(args: MeasureArgs) -> Result<MeasurementSummary> {
    let target = fingerprint_target(&args.target).await?;
    let profile = ProbeProfile::from(&args);

    if !args.refresh
        && let Some(summary) = cached_summary(&args.out, &target, &profile).await?
    {
        return Ok(summary);
    }

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
    let planner_stats = planner.stats();
    let run_context = ScoreRunContext {
        max_depth: planner_stats.max_depth,
        max_probes: args.max_probes,
        frontier_remaining: planner_stats.frontier_remaining,
        candidates_skipped_by_depth: planner_stats.candidates_skipped_by_depth,
    };
    let score_artifacts =
        score::write_score_artifacts(&args.out, target.clone(), &observations, run_context).await?;

    let summary = MeasurementSummary {
        target,
        probes_completed,
        evidence_path: args.out.join("evidence.jsonl"),
        shape_path: args.out.join("shape.json"),
        scorecard_path: score_artifacts.scorecard_path,
        report_path: score_artifacts.report_path,
        score_total: score_artifacts.total,
        score_measured_weight: score_artifacts.measured_weight,
        score_max_weight: score_artifacts.max_weight,
        score_model: score_artifacts.model.to_owned(),
        score_status: score_artifacts.status.to_owned(),
        findings: score_artifacts.findings,
        observed_max_depth: score_artifacts.observed_max_depth,
        max_depth: score_artifacts.max_depth,
        max_probes: score_artifacts.max_probes,
        frontier_remaining: score_artifacts.frontier_remaining,
        candidates_skipped_by_depth: score_artifacts.candidates_skipped_by_depth,
        probes_skipped_by_budget: score_artifacts.probes_skipped_by_budget,
        budget_exhausted: score_artifacts.budget_exhausted,
        cache_hit: false,
    };
    write_cache_manifest(&args.out, &summary, profile).await?;

    Ok(summary)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct ProbeProfile {
    timeout_ms: u64,
    output_limit_bytes: usize,
    max_depth: usize,
    max_probes: usize,
}

impl From<&MeasureArgs> for ProbeProfile {
    fn from(args: &MeasureArgs) -> Self {
        Self {
            timeout_ms: args.timeout_ms,
            output_limit_bytes: args.output_limit_bytes,
            max_depth: args.max_depth,
            max_probes: args.max_probes,
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
    findings: usize,
    observed_max_depth: usize,
    max_depth: usize,
    max_probes: usize,
    frontier_remaining: usize,
    candidates_skipped_by_depth: usize,
    probes_skipped_by_budget: usize,
    budget_exhausted: bool,
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
            score_total: self.summary.score_total,
            score_measured_weight: self.summary.score_measured_weight,
            score_max_weight: self.summary.score_max_weight,
            score_model: self.summary.score_model,
            score_status: self.summary.score_status,
            findings: self.summary.findings,
            observed_max_depth: self.summary.observed_max_depth,
            max_depth: self.summary.max_depth,
            max_probes: self.summary.max_probes,
            frontier_remaining: self.summary.frontier_remaining,
            candidates_skipped_by_depth: self.summary.candidates_skipped_by_depth,
            probes_skipped_by_budget: self.summary.probes_skipped_by_budget,
            budget_exhausted: self.summary.budget_exhausted,
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
            findings: summary.findings,
            observed_max_depth: summary.observed_max_depth,
            max_depth: summary.max_depth,
            max_probes: summary.max_probes,
            frontier_remaining: summary.frontier_remaining,
            candidates_skipped_by_depth: summary.candidates_skipped_by_depth,
            probes_skipped_by_budget: summary.probes_skipped_by_budget,
            budget_exhausted: summary.budget_exhausted,
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
    use std::path::PathBuf;

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
            score_total: 82.4,
            score_measured_weight: 0.9,
            score_max_weight: 1.0,
            score_model: "cliare-score-v0".to_owned(),
            score_status: "experimental partial".to_owned(),
            findings: 2,
            observed_max_depth: 1,
            max_depth: 5,
            max_probes: 256,
            frontier_remaining: 0,
            candidates_skipped_by_depth: 0,
            probes_skipped_by_budget: 0,
            budget_exhausted: false,
            cache_hit: false,
        };

        let text = summary.terminal_summary();

        assert!(text.contains("CLIARE measure complete"));
        assert!(text.contains("score: 82.4/100"));
        assert!(text.contains("cache: miss"));
        assert!(text.contains("depth: observed 1 / budget 5"));
        assert!(text.contains("  scorecard: .cliare/scorecard.json"));
        assert!(text.contains("  report: .cliare/report.md"));
    }
}
