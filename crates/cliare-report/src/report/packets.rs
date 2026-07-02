use super::actions::action_items;
use super::health::command_health;
use super::ledger::{reviewed_issues_for_persona, top_issues_for_persona};
use super::recommendations::{notes, run_recommendations};
use super::*;
use crate::report_evidence::EvidenceSummaryPacket;
use cliare_cli::cli::ReportArea;

impl PersonaOutcomePacket {
    pub(super) fn build(
        persona: Persona,
        artifact_dir: &Path,
        artifacts: &MeasuredArtifacts,
        issue_ledger: &IssueLedger,
    ) -> Self {
        let command_health = command_health(&artifacts.command_index);
        let summary = OutcomeSummary::from_artifacts(artifacts, command_health.len());
        let action_items = action_items(persona, artifacts);
        let run_recommendations = run_recommendations(persona, &artifacts.scorecard, artifact_dir);
        let notes = notes(persona, &artifacts.scorecard);
        let top_issues = top_issues_for_persona(persona, issue_ledger);
        let reviewed_issues = reviewed_issues_for_persona(persona, issue_ledger);

        Self {
            schema_version: PACKET_SCHEMA_VERSION,
            persona,
            persona_title: persona.title(),
            primary_question: persona.primary_question(),
            target: artifacts.scorecard.target.clone(),
            source_artifacts: SourceArtifacts::new(artifact_dir),
            summary,
            run_recommendations,
            top_issues,
            reviewed_issues,
            action_items,
            command_health,
            agent_navigation: AgentNavigationSection::from(&artifacts.scorecard.agent_navigation),
            score: ScoreSection::from(&artifacts.scorecard),
            coverage: CoverageSection::from(&artifacts.scorecard.coverage),
            evidence_summary: EvidenceSummaryPacket::from(&artifacts.evidence),
            notes,
        }
    }
}

impl ReportDrilldownPacket {
    pub(super) fn build(
        selection: ReportSelection,
        with_evidence: bool,
        persona: Persona,
        artifact_dir: &Path,
        artifacts: &MeasuredArtifacts,
        issue_ledger: &IssueLedger,
    ) -> Result<Self> {
        let command_health = command_health(&artifacts.command_index);
        let summary = OutcomeSummary::from_artifacts(artifacts, command_health.len());
        let filter = selection.filter();
        let mut issues = selection.select(persona, &issue_ledger.issues);
        if issues.is_empty() {
            return Err(CliareError::ReportFilterNoMatch {
                message: format!(
                    "no {} report issues matched {} `{}`",
                    persona.label(),
                    filter.kind.label(),
                    filter.value
                ),
            });
        }
        if !with_evidence {
            for issue in &mut issues {
                issue.evidence.clear();
            }
        }

        Ok(Self {
            schema_version: DRILLDOWN_SCHEMA_VERSION,
            persona,
            persona_title: persona.title(),
            primary_question: persona.primary_question(),
            target: artifacts.scorecard.target.clone(),
            source_artifacts: SourceArtifacts::new(artifact_dir),
            summary,
            filter,
            evidence_included: with_evidence,
            issues,
        })
    }
}

#[derive(Debug, Clone)]
pub(super) enum ReportSelection {
    Area(AgentReadinessArea),
    Issue(String),
}

impl ReportSelection {
    pub(super) fn from_args(args: &ReportArgs) -> Option<Self> {
        args.area
            .map(|area| Self::Area(AgentReadinessArea::from(area)))
            .or_else(|| args.issue.clone().map(Self::Issue))
    }

    fn filter(&self) -> ReportDrilldownFilter {
        match self {
            Self::Area(area) => ReportDrilldownFilter {
                kind: ReportDrilldownFilterKind::Area,
                value: area.slug().to_owned(),
            },
            Self::Issue(issue) => ReportDrilldownFilter {
                kind: ReportDrilldownFilterKind::Issue,
                value: issue.clone(),
            },
        }
    }

    pub(super) fn select(&self, persona: Persona, issues: &[Issue]) -> Vec<Issue> {
        issues
            .iter()
            .filter(|issue| match self {
                Self::Area(area) => {
                    issue.personas.contains(&persona) && issue.agent_readiness_area == *area
                }
                Self::Issue(id) => issue.id == *id,
            })
            .cloned()
            .collect()
    }
}

impl From<ReportArea> for AgentReadinessArea {
    fn from(value: ReportArea) -> Self {
        match value {
            ReportArea::OutputContracts => Self::OutputContracts,
            ReportArea::Preconditions => Self::Preconditions,
            ReportArea::CommandDiscovery => Self::CommandDiscovery,
            ReportArea::HelpCoverage => Self::HelpCoverage,
            ReportArea::Compatibility => Self::Compatibility,
            ReportArea::Diagnostics => Self::Diagnostics,
            ReportArea::Execution => Self::Execution,
            ReportArea::Safety => Self::Safety,
            ReportArea::Coverage => Self::Coverage,
            ReportArea::Policy => Self::Policy,
            ReportArea::Publishing => Self::Publishing,
            ReportArea::Calibration => Self::Calibration,
        }
    }
}

impl ReportDrilldownFilterKind {
    fn label(self) -> &'static str {
        match self {
            Self::Area => "area",
            Self::Issue => "issue",
        }
    }
}

impl OutcomeSummary {
    fn from_artifacts(artifacts: &MeasuredArtifacts, command_health_entries: usize) -> Self {
        let score = &artifacts.scorecard.score;
        let coverage = &artifacts.scorecard.coverage;
        Self {
            score: score.total,
            score_model: score.model.clone(),
            score_status: score.status.clone(),
            measured_weight: score.measured_weight,
            max_weight: score.max_weight,
            findings: artifacts.scorecard.findings.len(),
            shape_gaps: artifacts.shape.gaps.len(),
            commands_discovered: coverage.commands_discovered,
            commands_runtime_confirmed: coverage.commands_runtime_confirmed,
            commands_precondition_blocked: coverage.commands_precondition_blocked,
            command_health_entries,
            commands_ready: artifacts.command_index.summary.ready,
            commands_conditional: artifacts.command_index.summary.conditional,
            commands_needs_fixture: artifacts.command_index.summary.needs_fixture,
            commands_blocked: artifacts.command_index.summary.blocked,
            commands_candidate: artifacts.command_index.summary.candidate,
            output_contracts_discovered: coverage.output_contracts_discovered,
            machine_readable_output_contracts: coverage.machine_readable_output_contracts,
            output_mode_parse_successes: coverage.output_mode_parse_successes,
            side_effect_files_total: coverage.side_effect_files_total,
            credential_like_side_effects: coverage.credential_like_side_effects,
            precondition_blocked_probes: coverage.precondition_blocked_probes,
            auth_required_probes: coverage.auth_required_probes,
            local_context_required_probes: coverage.local_context_required_probes,
            fixture_required_probes: coverage.fixture_required_probes,
            observed_max_depth: coverage.observed_max_depth,
            max_depth: coverage.max_depth,
            max_probes: coverage.max_probes,
            probes_completed: coverage.probes_completed,
            frontier_remaining: coverage.frontier_remaining,
            budget_exhausted: coverage.budget_exhausted,
            traversal_stop_reason: coverage.traversal_stop_reason.clone(),
            traversal_complete: coverage.traversal_complete,
        }
    }
}

impl From<&AgentNavigationArtifact> for AgentNavigationSection {
    fn from(agent_navigation: &AgentNavigationArtifact) -> Self {
        Self {
            status: agent_navigation.status.clone(),
            dimensions: agent_navigation
                .dimensions
                .iter()
                .map(|(capability, metric)| {
                    (
                        capability.clone(),
                        AgentNavigationMetricPacket {
                            score: metric.score,
                            numerator: metric.numerator,
                            denominator: metric.denominator,
                            status: metric.status.clone(),
                            rationale: metric.rationale.clone(),
                            evidence: metric.evidence.clone(),
                            limitations: metric.limitations.clone(),
                        },
                    )
                })
                .collect(),
            limitations: agent_navigation.limitations.clone(),
        }
    }
}

impl From<&ScorecardArtifact> for ScoreSection {
    fn from(scorecard: &ScorecardArtifact) -> Self {
        Self {
            total: scorecard.score.total,
            measured_weight: scorecard.score.measured_weight,
            max_weight: scorecard.score.max_weight,
            model: scorecard.score.model.clone(),
            status: scorecard.score.status.clone(),
            subscores: scorecard
                .subscores
                .iter()
                .map(|(dimension, subscore)| {
                    (
                        dimension.clone(),
                        ScoreSubscorePacket {
                            score: subscore.score,
                            weight: subscore.weight,
                            status: subscore.status.clone(),
                            rationale: subscore.rationale.clone(),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl From<&CoverageArtifact> for CoverageSection {
    fn from(coverage: &CoverageArtifact) -> Self {
        Self {
            commands_discovered: coverage.commands_discovered,
            commands_runtime_confirmed: coverage.commands_runtime_confirmed,
            commands_precondition_blocked: coverage.commands_precondition_blocked,
            command_confirmation_rate: coverage.command_confirmation_rate,
            flags_discovered: coverage.flags_discovered,
            output_contracts_discovered: coverage.output_contracts_discovered,
            machine_readable_output_contracts: coverage.machine_readable_output_contracts,
            output_mode_probes_completed: coverage.output_mode_probes_completed,
            output_mode_parse_successes: coverage.output_mode_parse_successes,
            output_mode_precondition_blocked: coverage.output_mode_precondition_blocked,
            side_effect_files_created: coverage.side_effect_files_created,
            side_effect_files_modified: coverage.side_effect_files_modified,
            side_effect_files_deleted: coverage.side_effect_files_deleted,
            side_effect_files_total: coverage.side_effect_files_total,
            side_effect_probe_count: coverage.side_effect_probe_count,
            credential_like_side_effects: coverage.credential_like_side_effects,
            avg_command_confidence: coverage.avg_command_confidence,
            avg_flag_confidence: coverage.avg_flag_confidence,
            observed_max_depth: coverage.observed_max_depth,
            traversal_profile: coverage.traversal_profile.clone(),
            max_depth: coverage.max_depth,
            max_probes: coverage.max_probes,
            min_expected_value: coverage.min_expected_value,
            concurrency_limit: coverage.concurrency_limit,
            traversal_rounds: coverage.traversal_rounds,
            probes_scheduled: coverage.probes_scheduled,
            probes_completed: coverage.probes_completed,
            probes_cancelled: coverage.probes_cancelled,
            probes_timed_out: coverage.probes_timed_out,
            probes_failed_to_spawn: coverage.probes_failed_to_spawn,
            precondition_blocked_probes: coverage.precondition_blocked_probes,
            auth_required_probes: coverage.auth_required_probes,
            local_context_required_probes: coverage.local_context_required_probes,
            fixture_required_probes: coverage.fixture_required_probes,
            frontier_remaining: coverage.frontier_remaining,
            highest_pending_expected_value: coverage.highest_pending_expected_value,
            candidates_skipped_by_depth: coverage.candidates_skipped_by_depth,
            candidates_skipped_by_convergence: coverage.candidates_skipped_by_convergence,
            probes_skipped_by_budget: coverage.probes_skipped_by_budget,
            budget_exhausted: coverage.budget_exhausted,
            traversal_stop_reason: coverage.traversal_stop_reason.clone(),
            traversal_complete: coverage.traversal_complete,
        }
    }
}

impl ActionSeverity {
    pub(super) fn from_scorecard(value: &str) -> Self {
        match value {
            "high" => Self::High,
            "medium" => Self::Medium,
            _ => Self::Low,
        }
    }
}

impl ActionCategory {
    pub(super) fn from_dimension(value: &str) -> Self {
        match value {
            "discovery" => Self::Discovery,
            "grammar" => Self::Grammar,
            "execution" => Self::Execution,
            "output" => Self::Output,
            "safety" => Self::Safety,
            "recovery" => Self::Recovery,
            _ => Self::Coverage,
        }
    }
}
