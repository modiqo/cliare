use super::*;
use std::collections::{BTreeMap, BTreeSet};

use super::actions::action_items;
use super::issue_builder::issue_from_action_item;
use super::recommendations::persona_priority;

impl IssueLedger {
    pub(super) fn build(artifact_dir: &Path, artifacts: &MeasuredArtifacts) -> Self {
        let command_index = artifacts
            .shape
            .commands
            .iter()
            .map(|command| (command.path.clone(), command))
            .collect::<BTreeMap<_, _>>();
        let mut output_contracts_by_command =
            BTreeMap::<Vec<String>, Vec<&ShapeOutputContract>>::new();
        for contract in &artifacts.shape.output_contracts {
            output_contracts_by_command
                .entry(contract.command_path.clone())
                .or_default()
                .push(contract);
        }
        let mut gaps_by_command = BTreeMap::<Vec<String>, Vec<&ShapeGap>>::new();
        for gap in &artifacts.shape.gaps {
            gaps_by_command
                .entry(gap.command_path.clone())
                .or_default()
                .push(gap);
        }

        let mut issues = action_items(Persona::Maintainer, artifacts)
            .into_iter()
            .map(|item| {
                issue_from_action_item(
                    item,
                    artifact_dir,
                    artifacts,
                    &command_index,
                    &gaps_by_command,
                    &output_contracts_by_command,
                )
            })
            .collect::<Vec<_>>();
        issues.sort_by(|left, right| {
            left.severity
                .cmp(&right.severity)
                .then(left.category.cmp(&right.category))
                .then(left.id.cmp(&right.id))
        });

        let summary = IssueLedgerSummary::from_issues(&issues);
        Self {
            schema_version: ISSUE_LEDGER_SCHEMA_VERSION,
            target: artifacts.scorecard.target.clone(),
            source_artifacts: SourceArtifacts::new(artifact_dir),
            summary,
            issues,
        }
    }

    pub(super) fn apply_dispositions(&mut self, dispositions: &IssueDispositions) {
        let disposition_by_id = dispositions.by_issue_id();
        for issue in &mut self.issues {
            if let Some(disposition) = disposition_by_id.get(issue.id.as_str()) {
                issue.disposition = Some((*disposition).clone());
            }
        }
        self.summary = IssueLedgerSummary::from_issues(&self.issues);
    }
}

impl IssueLedgerSummary {
    fn from_issues(issues: &[Issue]) -> Self {
        let mut affected_commands = BTreeSet::<Vec<String>>::new();
        let mut high = 0_usize;
        let mut medium = 0_usize;
        let mut low = 0_usize;
        let mut requires_fixtures = 0_usize;
        let mut blocked_by_preconditions = 0_usize;
        let mut dispositioned = 0_usize;
        let mut action_required = 0_usize;
        let mut reviewed_decisions = 0_usize;

        for issue in issues {
            match issue.severity {
                ActionSeverity::High => high += 1,
                ActionSeverity::Medium => medium += 1,
                ActionSeverity::Low => low += 1,
            }
            if issue.confidence == IssueConfidence::NeedsFixture {
                requires_fixtures += 1;
            }
            if issue.confidence == IssueConfidence::Blocked {
                blocked_by_preconditions += 1;
            }
            if issue_action_required(issue) {
                action_required += 1;
            }
            if let Some(disposition) = &issue.disposition {
                dispositioned += 1;
                if !disposition.status.is_action_required() {
                    reviewed_decisions += 1;
                }
            }
            for command in &issue.affected_commands {
                affected_commands.insert(command.path.clone());
            }
        }

        Self {
            issues_total: issues.len(),
            high,
            medium,
            low,
            affected_commands: affected_commands.len(),
            requires_fixtures,
            blocked_by_preconditions,
            dispositioned,
            action_required,
            reviewed_decisions,
        }
    }
}

pub(super) fn top_issues_for_persona(persona: Persona, issue_ledger: &IssueLedger) -> Vec<Issue> {
    let mut issues = issue_ledger
        .issues
        .iter()
        .filter(|issue| issue.personas.contains(&persona) && issue_action_required(issue))
        .cloned()
        .collect::<Vec<_>>();
    issues.sort_by(|left, right| {
        persona_priority(persona, left.category)
            .cmp(&persona_priority(persona, right.category))
            .then(left.severity.cmp(&right.severity))
            .then(left.id.cmp(&right.id))
    });
    issues.truncate(TOP_ISSUE_LIMIT);
    issues
}

pub(super) fn reviewed_issues_for_persona(
    persona: Persona,
    issue_ledger: &IssueLedger,
) -> Vec<Issue> {
    let mut issues = issue_ledger
        .issues
        .iter()
        .filter(|issue| issue.personas.contains(&persona) && !issue_action_required(issue))
        .cloned()
        .collect::<Vec<_>>();
    issues.sort_by(|left, right| {
        left.disposition
            .as_ref()
            .map(|entry| entry.status)
            .cmp(&right.disposition.as_ref().map(|entry| entry.status))
            .then(left.id.cmp(&right.id))
    });
    issues.truncate(TOP_ISSUE_LIMIT);
    issues
}

fn issue_action_required(issue: &Issue) -> bool {
    issue
        .disposition
        .as_ref()
        .is_none_or(|disposition| disposition.status.is_action_required())
}
