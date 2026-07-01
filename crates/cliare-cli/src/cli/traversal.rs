use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub const QUICK_MAX_DEPTH: usize = 3;
pub const QUICK_MAX_PROBES: usize = 64;
pub const QUICK_MIN_EXPECTED_VALUE: u16 = 300;
pub const QUICK_CONCURRENCY: usize = 2;
pub const STANDARD_MAX_DEPTH: usize = 5;
pub const STANDARD_MAX_PROBES: usize = 256;
pub const STANDARD_MIN_EXPECTED_VALUE: u16 = 150;
pub const STANDARD_CONCURRENCY: usize = 4;
pub const DEEP_MAX_DEPTH: usize = 8;
pub const DEEP_MAX_PROBES: usize = 1_000;
pub const DEEP_MIN_EXPECTED_VALUE: u16 = 50;
pub const DEEP_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TraversalProfile {
    Quick,
    Standard,
    Deep,
}

impl TraversalProfile {
    pub fn default_max_depth(self) -> usize {
        match self {
            Self::Quick => QUICK_MAX_DEPTH,
            Self::Standard => STANDARD_MAX_DEPTH,
            Self::Deep => DEEP_MAX_DEPTH,
        }
    }

    pub fn default_max_probes(self) -> usize {
        match self {
            Self::Quick => QUICK_MAX_PROBES,
            Self::Standard => STANDARD_MAX_PROBES,
            Self::Deep => DEEP_MAX_PROBES,
        }
    }

    pub fn default_min_expected_value(self) -> u16 {
        match self {
            Self::Quick => QUICK_MIN_EXPECTED_VALUE,
            Self::Standard => STANDARD_MIN_EXPECTED_VALUE,
            Self::Deep => DEEP_MIN_EXPECTED_VALUE,
        }
    }

    pub fn default_concurrency(self) -> usize {
        match self {
            Self::Quick => QUICK_CONCURRENCY,
            Self::Standard => STANDARD_CONCURRENCY,
            Self::Deep => DEEP_CONCURRENCY,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }
}

impl std::fmt::Display for TraversalProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

pub(crate) fn parse_positive_usize(raw: &str) -> std::result::Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|source| format!("expected positive integer: {source}"))?;
    if value == 0 {
        Err("expected positive integer greater than zero".to_owned())
    } else {
        Ok(value)
    }
}

pub(crate) fn parse_positive_u64(raw: &str) -> std::result::Result<u64, String> {
    let value = raw
        .parse::<u64>()
        .map_err(|source| format!("expected positive integer: {source}"))?;
    if value == 0 {
        Err("expected positive integer greater than zero".to_owned())
    } else {
        Ok(value)
    }
}
