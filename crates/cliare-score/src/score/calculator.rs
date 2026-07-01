use cliare_inference::score_model::ScoreModelSpec;
use cliare_runtime::fingerprint::TargetFingerprint;
use cliare_shape::claims::ClaimSet;
use cliare_shape::observation::ShapeObservation;

use super::SCHEMA_VERSION;
use super::findings::findings;
use super::formulas::{score_summary, subscores};
use super::labels::{normalization_label, score_status_label};
use super::metrics::Metrics;
use super::model::{ScoreModel, ScoreRunContext, Scorecard};
use super::util::target_binary_name;

pub fn scorecard(
    target: TargetFingerprint,
    observations: &[ShapeObservation],
    run_context: ScoreRunContext,
) -> Scorecard {
    let binary_name = target_binary_name(&target);
    let model_spec = ScoreModelSpec::bundled();
    let claims = ClaimSet::from_observations_with_model(&binary_name, observations, model_spec);
    let runtime_context = run_context.runtime_context.clone();
    let metrics =
        Metrics::from_claims_and_observations(&claims, &binary_name, observations, run_context);

    let subscores = subscores(&metrics, model_spec);
    let score = score_summary(&subscores, model_spec, &metrics);
    let score_status = score_status_label(&score.status).to_owned();
    let findings = findings(&metrics, model_spec);

    Scorecard {
        schema_version: SCHEMA_VERSION,
        target,
        runtime_context,
        score,
        subscores,
        coverage: metrics.coverage,
        findings,
        model: ScoreModel {
            name: model_spec.id.clone(),
            sha256: ScoreModelSpec::bundled_sha256().to_owned(),
            source: model_spec.source.clone(),
            status: score_status,
            normalization: normalization_label(model_spec.normalization).to_owned(),
        },
    }
}
