use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use tokio::fs;

use cliare_core::artifacts::ISSUE_DISPOSITIONS_JSON;
use cliare_core::error::{CliareError, Result};

pub const ISSUE_DISPOSITIONS_SCHEMA_VERSION: &str = "cliare.issue-dispositions.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum IssueDispositionStatus {
    Open,
    Accepted,
    Intentional,
    NotApplicable,
    FalsePositive,
    AcceptedRisk,
    NeedsFixture,
    Deferred,
}

impl IssueDispositionStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Accepted => "accepted",
            Self::Intentional => "intentional",
            Self::NotApplicable => "not_applicable",
            Self::FalsePositive => "false_positive",
            Self::AcceptedRisk => "accepted_risk",
            Self::NeedsFixture => "needs_fixture",
            Self::Deferred => "deferred",
        }
    }

    pub fn is_action_required(self) -> bool {
        matches!(self, Self::Open | Self::Accepted | Self::NeedsFixture)
    }
}

impl std::fmt::Display for IssueDispositionStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct IssueDisposition {
    pub issue_id: String,
    pub status: IssueDispositionStatus,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IssueDispositions {
    pub schema_version: String,
    pub dispositions: Vec<IssueDisposition>,
}

impl Default for IssueDispositions {
    fn default() -> Self {
        Self {
            schema_version: ISSUE_DISPOSITIONS_SCHEMA_VERSION.to_owned(),
            dispositions: Vec::new(),
        }
    }
}

impl IssueDispositions {
    pub async fn read_optional(artifact_dir: &Path) -> Result<Self> {
        let path = disposition_path(artifact_dir);
        match fs::read(&path).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|source| CliareError::ParseIssueDispositions { path, source }),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(CliareError::ReadIssueDispositions { path, source }),
        }
    }

    pub async fn write(&self, artifact_dir: &Path) -> Result<PathBuf> {
        let path = disposition_path(artifact_dir);
        let bytes =
            serde_json::to_vec_pretty(self).map_err(CliareError::SerializeIssueDispositions)?;
        fs::write(&path, bytes)
            .await
            .map_err(|source| CliareError::WriteIssueDispositions {
                path: path.clone(),
                source,
            })?;
        Ok(path)
    }

    pub fn mark(&mut self, issue_id: String, status: IssueDispositionStatus, reason: String) {
        match self
            .dispositions
            .iter_mut()
            .find(|entry| entry.issue_id == issue_id)
        {
            Some(entry) => {
                entry.status = status;
                entry.reason = reason;
            }
            None => self.dispositions.push(IssueDisposition {
                issue_id,
                status,
                reason,
            }),
        }
        self.dispositions
            .sort_by(|left, right| left.issue_id.cmp(&right.issue_id));
    }

    pub fn by_issue_id(&self) -> BTreeMap<&str, &IssueDisposition> {
        self.dispositions
            .iter()
            .map(|entry| (entry.issue_id.as_str(), entry))
            .collect()
    }
}

pub fn disposition_path(artifact_dir: &Path) -> PathBuf {
    artifact_dir.join(ISSUE_DISPOSITIONS_JSON)
}

#[cfg(test)]
mod tests {
    use super::{IssueDispositionStatus, IssueDispositions};

    #[test]
    fn mark_replaces_existing_disposition() {
        let mut dispositions = IssueDispositions::default();

        dispositions.mark(
            "issue.test".to_owned(),
            IssueDispositionStatus::Accepted,
            "first".to_owned(),
        );
        dispositions.mark(
            "issue.test".to_owned(),
            IssueDispositionStatus::Intentional,
            "second".to_owned(),
        );

        assert_eq!(dispositions.dispositions.len(), 1);
        assert_eq!(
            dispositions.dispositions[0].status,
            IssueDispositionStatus::Intentional
        );
        assert_eq!(dispositions.dispositions[0].reason, "second");
    }
}
