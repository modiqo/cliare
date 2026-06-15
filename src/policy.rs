use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use tokio::fs;

use crate::artifacts::{EVIDENCE_JSONL, SCORECARD_JSON};
use crate::error::{CliareError, Result};
use crate::path_classification;

const POLICY_SCHEMA_VERSION: &str = "cliare.policy.v1";

#[derive(Debug, Clone)]
pub struct PolicyEvaluation {
    pub policy_path: PathBuf,
    pub passed: bool,
    pub failures: Vec<PolicyFailure>,
}

#[derive(Debug, Clone)]
pub struct PolicyFailure {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub recommendation: String,
}

pub async fn evaluate_policy(policy_path: &Path, out_dir: &Path) -> Result<PolicyEvaluation> {
    let policy = read_policy(policy_path).await?;
    let scorecard = read_scorecard(&out_dir.join(SCORECARD_JSON)).await?;
    let side_effects = read_side_effects(&out_dir.join(EVIDENCE_JSONL)).await?;
    let mut failures = Vec::new();

    evaluate_score_thresholds(&policy, &scorecard, &mut failures)?;
    evaluate_side_effects(&policy, &side_effects, &mut failures);

    Ok(PolicyEvaluation {
        policy_path: policy_path.to_path_buf(),
        passed: failures.is_empty(),
        failures,
    })
}

async fn read_policy(path: &Path) -> Result<Policy> {
    let bytes = fs::read(path)
        .await
        .map_err(|source| CliareError::ReadPolicy {
            path: path.to_path_buf(),
            source,
        })?;
    let policy: Policy =
        serde_json::from_slice(&bytes).map_err(|source| CliareError::ParsePolicy {
            path: path.to_path_buf(),
            source,
        })?;
    if policy.schema_version != POLICY_SCHEMA_VERSION {
        return Err(CliareError::UnsupportedPolicySchema {
            path: path.to_path_buf(),
            schema_version: policy.schema_version,
        });
    }
    Ok(policy)
}

async fn read_scorecard(path: &Path) -> Result<PolicyScorecard> {
    let bytes = fs::read(path)
        .await
        .map_err(|source| CliareError::ReadPolicyScorecard {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::from_slice(&bytes).map_err(|source| CliareError::ParsePolicyScorecard {
        path: path.to_path_buf(),
        source,
    })
}

async fn read_side_effects(path: &Path) -> Result<Vec<SideEffectChange>> {
    let text =
        fs::read_to_string(path)
            .await
            .map_err(|source| CliareError::ReadPolicyEvidence {
                path: path.to_path_buf(),
                source,
            })?;
    let mut changes = Vec::new();

    for (line_index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value =
            serde_json::from_str(line).map_err(|source| CliareError::ParsePolicyEvidence {
                path: path.to_path_buf(),
                line: line_index + 1,
                source,
            })?;
        if value["kind"].as_str() != Some("process_completed") {
            continue;
        }
        let Some(raw_changes) = value["payload"]["side_effects"]["changes"].as_array() else {
            continue;
        };
        for raw_change in raw_changes {
            let Some(path) = raw_change["path"].as_str() else {
                continue;
            };
            changes.push(SideEffectChange {
                path: normalize_path(path),
            });
        }
    }

    Ok(changes)
}

fn evaluate_score_thresholds(
    policy: &Policy,
    scorecard: &PolicyScorecard,
    failures: &mut Vec<PolicyFailure>,
) -> Result<()> {
    if let Some(minimum) = policy.min_total_score {
        validate_score_threshold(minimum, "min_total_score")?;
        if scorecard.score.total < minimum {
            failures.push(PolicyFailure {
                id: "policy.score.total_minimum".to_owned(),
                title: "Total score is below policy minimum".to_owned(),
                detail: format!(
                    "Total score {:.1} is below required minimum {:.1}.",
                    scorecard.score.total, minimum
                ),
                recommendation: "Improve CLI readiness or lower the explicit policy threshold."
                    .to_owned(),
            });
        }
    }

    for (dimension, minimum) in &policy.min_subscores {
        validate_score_threshold(*minimum, &format!("min_subscores.{dimension}"))?;
        match scorecard
            .subscores
            .get(dimension)
            .and_then(|subscore| subscore.score)
        {
            Some(actual) if actual >= *minimum => {}
            Some(actual) => failures.push(PolicyFailure {
                id: format!("policy.score.subscore_minimum.{dimension}"),
                title: format!("Policy subscore minimum failed for {dimension}"),
                detail: format!(
                    "{dimension} score {:.1} is below required minimum {:.1}.",
                    actual, minimum
                ),
                recommendation:
                    "Improve this measured dimension or lower the explicit policy threshold."
                        .to_owned(),
            }),
            None => failures.push(PolicyFailure {
                id: format!("policy.score.subscore_missing.{dimension}"),
                title: format!("Policy subscore was not measured for {dimension}"),
                detail: format!(
                    "Policy requires {dimension} score at least {:.1}, but that subscore was absent or not measured.",
                    minimum
                ),
                recommendation: "Measure a scorecard that includes this dimension or remove the policy threshold."
                    .to_owned(),
            }),
        }
    }

    Ok(())
}

fn evaluate_side_effects(
    policy: &Policy,
    side_effects: &[SideEffectChange],
    failures: &mut Vec<PolicyFailure>,
) {
    let Some(rule) = &policy.side_effects else {
        return;
    };
    let unapproved = side_effects
        .iter()
        .filter(|change| !rule.allows(&change.path))
        .collect::<Vec<_>>();
    let effective_max = rule.max_unapproved.or({
        if rule.allow_paths.is_empty() {
            None
        } else {
            Some(0)
        }
    });

    if let Some(max_unapproved) = effective_max
        && unapproved.len() > max_unapproved
    {
        failures.push(PolicyFailure {
            id: "policy.side_effects.unapproved".to_owned(),
            title: "Unapproved side effects exceeded policy limit".to_owned(),
            detail: format!(
                "{} unapproved side-effect path(s) were observed; policy allows {}.{}",
                unapproved.len(),
                max_unapproved,
                sample_paths(&unapproved)
            ),
            recommendation:
                "Add explicit allow_paths entries for expected safe writes or remove probe-time writes."
                    .to_owned(),
        });
    }

    if rule.deny_credential_like {
        let credential_like = side_effects
            .iter()
            .filter(|change| path_classification::credential_like_path_text(&change.path))
            .collect::<Vec<_>>();
        if !credential_like.is_empty() {
            failures.push(PolicyFailure {
                id: "policy.side_effects.credential_like".to_owned(),
                title: "Credential-like side effects are denied by policy".to_owned(),
                detail: format!(
                    "{} credential-like side-effect path(s) were observed.{}",
                    credential_like.len(),
                    sample_paths(&credential_like)
                ),
                recommendation:
                    "Do not create token, secret, credential, or key material during discovery probes."
                        .to_owned(),
            });
        }
    }
}

fn validate_score_threshold(value: f64, field: &str) -> Result<()> {
    if value.is_finite() && (0.0..=100.0).contains(&value) {
        Ok(())
    } else {
        Err(CliareError::InvalidPolicyScoreThreshold {
            field: field.to_owned(),
            value,
        })
    }
}

fn sample_paths(changes: &[&SideEffectChange]) -> String {
    if changes.is_empty() {
        return String::new();
    }
    let mut paths = changes
        .iter()
        .take(5)
        .map(|change| change.path.clone())
        .collect::<Vec<_>>();
    paths.sort();
    format!(" Paths: {}.", paths.join(", "))
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_owned()
}

fn glob_matches(pattern: &str, path: &str) -> bool {
    let pattern_segments = normalize_path(pattern)
        .split('/')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let path_segments = normalize_path(path)
        .split('/')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    glob_segments_match(&pattern_segments, &path_segments)
}

fn glob_segments_match(pattern: &[String], path: &[String]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((segment, rest)) if segment == "**" => {
            glob_segments_match(rest, path)
                || (!path.is_empty() && glob_segments_match(pattern, &path[1..]))
        }
        Some((segment, rest)) => {
            !path.is_empty()
                && segment_matches(segment, &path[0])
                && glob_segments_match(rest, &path[1..])
        }
    }
}

fn segment_matches(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let parts = pattern.split('*').collect::<Vec<_>>();
    if parts.len() == 1 {
        return pattern == text;
    }

    let mut remaining = text;
    if let Some(first) = parts.first()
        && !first.is_empty()
    {
        let Some(after_prefix) = remaining.strip_prefix(first) else {
            return false;
        };
        remaining = after_prefix;
    }

    for part in parts
        .iter()
        .skip(1)
        .take(parts.len().saturating_sub(2))
        .filter(|part| !part.is_empty())
    {
        let Some(index) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[index + part.len()..];
    }

    if let Some(last) = parts.last()
        && !last.is_empty()
    {
        return remaining.ends_with(last);
    }

    true
}

#[derive(Debug, Deserialize)]
struct Policy {
    schema_version: String,
    min_total_score: Option<f64>,
    #[serde(default)]
    min_subscores: BTreeMap<String, f64>,
    side_effects: Option<SideEffectPolicy>,
}

#[derive(Debug, Deserialize)]
struct SideEffectPolicy {
    #[serde(default)]
    allow_paths: Vec<String>,
    max_unapproved: Option<usize>,
    #[serde(default)]
    deny_credential_like: bool,
}

impl SideEffectPolicy {
    fn allows(&self, path: &str) -> bool {
        self.allow_paths
            .iter()
            .any(|pattern| glob_matches(pattern, path))
    }
}

#[derive(Debug, Deserialize)]
struct PolicyScorecard {
    score: PolicyScore,
    subscores: BTreeMap<String, PolicySubscore>,
}

#[derive(Debug, Deserialize)]
struct PolicyScore {
    total: f64,
}

#[derive(Debug, Deserialize)]
struct PolicySubscore {
    score: Option<f64>,
}

#[derive(Debug)]
struct SideEffectChange {
    path: String,
}

#[cfg(test)]
mod tests {
    use super::glob_matches;
    use crate::path_classification;

    #[test]
    fn glob_matches_single_and_recursive_segments() {
        assert!(glob_matches(
            "xdg-cache/fixture-cli/*",
            "xdg-cache/fixture-cli/help-cache"
        ));
        assert!(!glob_matches(
            "xdg-cache/fixture-cli/*",
            "xdg-cache/fixture-cli/nested/help-cache"
        ));
        assert!(glob_matches(
            "xdg-cache/**",
            "xdg-cache/fixture-cli/nested/help-cache"
        ));
        assert!(glob_matches("home/*-token", "home/session-token"));
    }

    #[test]
    fn credential_like_paths_match_score_heuristic_terms() {
        assert!(path_classification::credential_like_path_text(
            "home/session-token"
        ));
        assert!(path_classification::credential_like_path_text(
            "home/run-secret"
        ));
        assert!(!path_classification::credential_like_path_text(
            "xdg-cache/fixture-cli/help-cache"
        ));
    }
}
