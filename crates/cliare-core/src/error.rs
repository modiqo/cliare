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
    #[error("failed to read context workdir {path}")]
    ReadContextWorkdir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("context workdir is not a directory: {0}")]
    ContextWorkdirNotDirectory(PathBuf),
    #[error("failed to open evidence log {path}")]
    OpenEvidenceLog {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write evidence event")]
    WriteEvidence(#[source] std::io::Error),
    #[error("failed to commit evidence log {path}")]
    CommitEvidence {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
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
    #[error("failed to read issue ledger {path}")]
    ReadIssueLedger {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse issue ledger {path}")]
    ParseIssueLedger {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to read issue dispositions {path}")]
    ReadIssueDispositions {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse issue dispositions {path}")]
    ParseIssueDispositions {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize issue dispositions")]
    SerializeIssueDispositions(#[source] serde_json::Error),
    #[error("failed to serialize playbook")]
    SerializePlaybook(#[source] serde_json::Error),
    #[error("failed to write issue dispositions {path}")]
    WriteIssueDispositions {
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
    #[error("failed to remove stale measurement cache manifest {path}")]
    RemoveMeasurementCache {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to clean abandoned in-progress measurement artifact {path}")]
    CleanupInProgressArtifact {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read measurement checkpoint {path}")]
    ReadMeasurementCheckpoint {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse measurement checkpoint {path}")]
    ParseMeasurementCheckpoint {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize measurement checkpoint")]
    SerializeMeasurementCheckpoint(#[source] serde_json::Error),
    #[error("failed to write measurement checkpoint {path}")]
    WriteMeasurementCheckpoint {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to remove measurement checkpoint {path}")]
    RemoveMeasurementCheckpoint {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create progress directory {path}")]
    CreateProgressDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to open progress log {path}")]
    OpenProgressLog {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write progress log {path}")]
    WriteProgressLog {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to locate current CLIARE executable")]
    CurrentExecutable(#[source] std::io::Error),
    #[error("failed to open detached job stream {path}")]
    OpenDetachedJobStream {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to spawn detached measure job")]
    SpawnDetachedMeasure(#[source] std::io::Error),
    #[error("{message}")]
    DetachedJobAlreadyActive { message: String },
    #[error("failed to read job pointer {path}")]
    ReadJobPointer {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read job progress log {path}")]
    ReadJobProgressLog {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read detached job stream {path}")]
    ReadJobStream {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("artifact path is not a directory: {path}")]
    ArtifactPathNotDirectory { path: PathBuf },
    #[error("{command} could not find CLIARE measurement artifact directory {path}")]
    MeasurementArtifactNotFound { command: String, path: PathBuf },
    #[error("{message}")]
    InvalidMeasurementArtifact { message: String },
    #[error("failed to read artifact directory {path}")]
    ReadArtifactDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize artifact map")]
    SerializeArtifactMap(#[source] serde_json::Error),
    #[error("failed to write artifact map {path}")]
    WriteArtifactMap {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read command index {path}")]
    ReadCommandIndex {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse command index {path}")]
    ParseCommandIndex {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize surface query response")]
    SerializeSurface(#[source] serde_json::Error),
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
    #[error("failed to create benchmark directory {path}")]
    CreateBenchmarkDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read benchmark manifest {path}")]
    ReadBenchmarkManifest {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse benchmark manifest {path}")]
    ParseBenchmarkManifest {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported benchmark corpus schema {schema_version}")]
    UnsupportedBenchmarkSchema { schema_version: String },
    #[error("benchmark target {target_id} has invalid expected score band: min {min}, max {max}")]
    InvalidBenchmarkScoreBand {
        target_id: String,
        min: f64,
        max: f64,
    },
    #[error("benchmark field {field} must be greater than zero: {value}")]
    InvalidBenchmarkPositiveInteger { field: &'static str, value: usize },
    #[error("benchmark output directory is already locked by {path}")]
    BenchmarkOutputLocked { path: PathBuf },
    #[error("failed to acquire benchmark output lock {path}")]
    AcquireBenchmarkLock {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize benchmark report")]
    SerializeBenchmarkReport(#[source] serde_json::Error),
    #[error("failed to write benchmark report {path}")]
    WriteBenchmarkReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write benchmark markdown {path}")]
    WriteBenchmarkMarkdown {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize runtime context")]
    SerializeRuntimeContext(#[source] serde_json::Error),
    #[error("failed to write runtime context {path}")]
    WriteRuntimeContext {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create context suite directory {path}")]
    CreateContextSuiteDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read context suite directory {path}")]
    ReadContextSuiteDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read context scorecard {path}")]
    ReadContextScorecard {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse context scorecard {path}")]
    ParseContextScorecard {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize context suite")]
    SerializeContextSuite(#[source] serde_json::Error),
    #[error("failed to write context suite artifact {path}")]
    WriteContextSuite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{message}")]
    ContextSelectionRequired { message: String },
    #[error("{message}")]
    ContextSelectionNotFound { message: String },
    #[error("failed to read report artifact {path}")]
    ReadReportArtifact {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse report artifact {path}")]
    ParseReportArtifact {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to parse report evidence {path}:{line}")]
    ParseReportEvidence {
        path: PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize persona outcome packet")]
    SerializePersonaOutcome(#[source] serde_json::Error),
    #[error("{message}")]
    ReportFilterNoMatch { message: String },
    #[error("failed to write persona outcome packet {path}")]
    WritePersonaOutcome {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write artifact guide {path}")]
    WriteArtifactGuide {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("home directory is unavailable; pass --home for user-scope skill installation")]
    HomeDirectoryUnavailable,
    #[error("failed to resolve current directory")]
    CurrentDirectory(#[source] std::io::Error),
    #[error("failed to serialize skill catalog")]
    SerializeSkillCatalog(#[source] serde_json::Error),
    #[error("failed to create skill installation directory {path}")]
    CreateSkillDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read installed skill artifact {path}")]
    ReadInstalledSkill {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write installed skill artifact {path}")]
    WriteInstalledSkill {
        path: PathBuf,
        #[source]
        source: std::io::Error,
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
