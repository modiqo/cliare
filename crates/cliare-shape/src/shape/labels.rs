use super::model::{AgentSuitability, CommandRuntimeState, GapKind, OutputContractStatus};

pub(super) fn command_display(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(" ")
    }
}

pub(super) fn suitability_label(suitability: AgentSuitability) -> &'static str {
    match suitability {
        AgentSuitability::Ready => "ready",
        AgentSuitability::Conditional => "conditional",
        AgentSuitability::NeedsFixture => "needs_fixture",
        AgentSuitability::Blocked => "blocked",
        AgentSuitability::Candidate => "candidate",
    }
}

pub(super) fn runtime_state_label(state: CommandRuntimeState) -> &'static str {
    match state {
        CommandRuntimeState::RuntimeConfirmed => "runtime_confirmed",
        CommandRuntimeState::PreconditionBlocked => "precondition_blocked",
        CommandRuntimeState::Unconfirmed => "unconfirmed",
    }
}

pub(super) fn output_status_label(status: OutputContractStatus) -> &'static str {
    match status {
        OutputContractStatus::ParseSuccess => "parse_success",
        OutputContractStatus::PreconditionBlocked => "precondition_blocked",
        OutputContractStatus::Unprobed => "unprobed",
        OutputContractStatus::HelpText => "help_text",
        OutputContractStatus::Unvalidated => "unvalidated",
        OutputContractStatus::ParseFailed => "parse_failed",
    }
}

pub(super) fn gap_kind_label(kind: GapKind) -> &'static str {
    match kind {
        GapKind::ExistenceUnconfirmed => "existence_unconfirmed",
        GapKind::HelpUnavailable => "help_unavailable",
        GapKind::AlternateHelpFormUnavailable => "alternate_help_form_unavailable",
        GapKind::PreconditionBlocked => "precondition_blocked",
        GapKind::FlagsUnknown => "flags_unknown",
        GapKind::ArgumentArityUnknown => "argument_arity_unknown",
        GapKind::InvalidChildDiagnosticsUnknown => "invalid_child_diagnostics_unknown",
        GapKind::InvalidFlagDiagnosticsUnknown => "invalid_flag_diagnostics_unknown",
        GapKind::OutputModeUnprobed => "output_mode_unprobed",
        GapKind::OutputModeUnvalidated => "output_mode_unvalidated",
        GapKind::OutputModeParseFailed => "output_mode_parse_failed",
    }
}

pub(super) fn escape_markdown_table(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', " ")
}
