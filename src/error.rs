use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CliareError {
    #[error("target executable was not found: {0}")]
    TargetNotFound(PathBuf),

    #[error("target path is not a file: {0}")]
    TargetNotFile(PathBuf),

    #[error("failed to resolve target path {path}")]
    ResolveTarget {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to fingerprint target {path}")]
    Fingerprint {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create artifact directory {path}")]
    CreateArtifactDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create sandbox directory {path}")]
    CreateSandboxDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to clear sandbox directory {path}")]
    ClearSandboxDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read sandbox directory {path}")]
    ReadSandboxDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read sandbox metadata {path}")]
    ReadSandboxMetadata {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read sandbox file {path}")]
    ReadSandboxFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to open evidence log {path}")]
    OpenEvidenceLog {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write evidence event")]
    WriteEvidence(#[source] std::io::Error),

    #[error("failed to serialize evidence event")]
    SerializeEvidence(#[source] serde_json::Error),

    #[error("failed to serialize command shape")]
    SerializeShape(#[source] serde_json::Error),

    #[error("failed to write command shape {path}")]
    WriteShape {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to serialize scorecard")]
    SerializeScorecard(#[source] serde_json::Error),

    #[error("failed to write scorecard {path}")]
    WriteScorecard {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write report {path}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read scorecard for CI artifacts {path}")]
    ReadCiScorecard {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse scorecard for CI artifacts {path}")]
    ParseCiScorecard {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to write CI summary {path}")]
    WriteCiSummary {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to serialize SARIF")]
    SerializeSarif(#[source] serde_json::Error),

    #[error("failed to write SARIF {path}")]
    WriteSarif {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write JUnit XML {path}")]
    WriteJunit {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read measurement cache manifest {path}")]
    ReadMeasurementCache {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse measurement cache manifest {path}")]
    ParseMeasurementCache {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to serialize measurement cache manifest")]
    SerializeMeasurementCache(#[source] serde_json::Error),

    #[error("failed to write measurement cache manifest {path}")]
    WriteMeasurementCache {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read baseline scorecard {path}")]
    ReadBaselineScorecard {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse baseline scorecard {path}")]
    ParseBaselineScorecard {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to read policy file {path}")]
    ReadPolicy {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse policy file {path}")]
    ParsePolicy {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("unsupported policy schema {schema_version} in {path}")]
    UnsupportedPolicySchema {
        path: PathBuf,
        schema_version: String,
    },

    #[error("policy score threshold {field} must be finite and between 0 and 100: {value}")]
    InvalidPolicyScoreThreshold { field: String, value: f64 },

    #[error("failed to read scorecard for policy evaluation {path}")]
    ReadPolicyScorecard {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse scorecard for policy evaluation {path}")]
    ParsePolicyScorecard {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to read evidence for policy evaluation {path}")]
    ReadPolicyEvidence {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse evidence for policy evaluation {path}:{line}")]
    ParsePolicyEvidence {
        path: PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    #[error("baseline score must be finite and between 0 and 100: {total}")]
    InvalidBaselineScore { total: f64 },

    #[error("allowed score drop must be finite and non-negative: {value}")]
    InvalidAllowedDrop { value: f64 },

    #[error("failed to spawn target process")]
    Spawn(#[source] std::io::Error),

    #[error("spawned target process did not expose {stream} pipe")]
    MissingPipe { stream: &'static str },

    #[error("failed to read process output")]
    ReadOutput(#[source] std::io::Error),

    #[error("failed to wait for target process")]
    Wait(#[source] std::io::Error),

    #[error("process reader task failed")]
    Join(#[source] tokio::task::JoinError),

    #[error("failed to format timestamp")]
    TimeFormat(#[source] time::error::Format),
}

pub type Result<T> = std::result::Result<T, CliareError>;
