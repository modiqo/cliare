use crate::output::OutputMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateFlagSection {
    Command,
    Global,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateOutputModeScope {
    CommandFlag,
    GlobalFlag,
    Example,
}

#[derive(Debug, Clone)]
pub struct CandidateCommand {
    pub path: Vec<String>,
    pub aliases: Vec<String>,
    pub summary: Option<String>,
    pub absolute: bool,
    pub evidence_detail: String,
}

#[derive(Debug, Clone)]
pub struct CandidateFlag {
    pub name: String,
    pub short: Option<String>,
    pub invocation: String,
    pub summary: Option<String>,
    pub section: CandidateFlagSection,
    pub value_kind: CandidateFlagValueKind,
    pub value_name: Option<String>,
    pub required: bool,
    pub repeatable: bool,
    pub evidence_detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateOutputMode {
    pub mode: OutputMode,
    pub flag_name: String,
    pub argv_fragment: Vec<String>,
    pub scope: CandidateOutputModeScope,
    pub evidence_detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateFlagValueKind {
    Boolean,
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateArgument {
    pub name: String,
    pub required: bool,
    pub variadic: bool,
    pub evidence_detail: String,
}
