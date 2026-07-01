use cliare_runtime::fingerprint::TargetFingerprint;

pub(super) fn average(values: impl Iterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut count = 0_usize;

    for value in values {
        sum += value;
        count += 1;
    }

    if count == 0 { 0.0 } else { sum / count as f64 }
}

pub(super) fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

pub(super) fn round_score(score: f64) -> f64 {
    score.clamp(0.0, 100.0).round()
}

pub(super) fn round_weight(weight: f64) -> f64 {
    (weight * 100.0).round() / 100.0
}

pub(super) fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}
