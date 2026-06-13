use std::collections::BTreeMap;

use crate::belief::Belief;
use crate::evidence::{ProbeIntent, ProcessStatus};
use crate::layout;
use crate::observation::ShapeObservation;

const COMMAND_PRIOR: f64 = 0.08;
const FLAG_PRIOR: f64 = 0.12;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandPath(Vec<String>);

impl CommandPath {
    pub fn new(path: Vec<String>) -> Self {
        Self(path)
    }

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.0.clone()
    }
}

#[derive(Debug)]
pub struct ClaimSet {
    commands: BTreeMap<CommandPath, CommandClaim>,
    flags: BTreeMap<(CommandPath, String), FlagClaim>,
}

impl ClaimSet {
    pub fn from_observations(binary_name: &str, observations: &[ShapeObservation]) -> Self {
        let mut claims = Self {
            commands: BTreeMap::new(),
            flags: BTreeMap::new(),
        };

        for observation in observations {
            claims.apply_observation(binary_name, observation);
        }

        claims
    }

    pub fn commands(&self) -> impl Iterator<Item = &CommandClaim> {
        self.commands.values()
    }

    pub fn flags(&self) -> impl Iterator<Item = &FlagClaim> {
        self.flags.values()
    }

    fn apply_observation(&mut self, binary_name: &str, observation: &ShapeObservation) {
        match observation.intent {
            ProbeIntent::Help => self.apply_help_observation(binary_name, observation),
            ProbeIntent::InvalidChild => self.apply_invalid_child_observation(observation),
            ProbeIntent::InvalidFlag => self.apply_invalid_flag_observation(observation),
            ProbeIntent::Version | ProbeIntent::InvalidCommand => {}
        }
    }

    fn apply_help_observation(&mut self, binary_name: &str, observation: &ShapeObservation) {
        let help_like = successful_help_like(observation);
        let current_path = CommandPath::new(observation.path.clone());

        if !current_path.is_empty() {
            self.command_mut(current_path.clone())
                .apply_runtime_help(&observation.evidence_id, help_like);
        }

        let Some(text) = observation.process.stdout.text.as_deref() else {
            return;
        };
        if !help_like && !current_path.is_empty() {
            return;
        }

        for candidate in layout::command_candidates(text, binary_name) {
            let path = CommandPath::new(absolutize_candidate_path(
                current_path.as_slice(),
                candidate.path,
            ));
            if path == current_path {
                continue;
            }
            if is_child_path(current_path.as_slice(), path.as_slice()) {
                self.command_mut(current_path.clone())
                    .apply_child_candidate(&observation.evidence_id);
            }

            self.command_mut(path).apply_layout_candidate(
                candidate.summary,
                &observation.evidence_id,
                &candidate.evidence_detail,
            );
        }

        for candidate in layout::flag_candidates(text) {
            let key = (current_path.clone(), candidate.name.clone());
            self.flags
                .entry(key)
                .or_insert_with(|| FlagClaim::new(current_path.clone(), candidate.name.clone()))
                .apply_layout_candidate(
                    candidate.short,
                    candidate.summary,
                    &observation.evidence_id,
                    &candidate.evidence_detail,
                );
        }
    }

    fn apply_invalid_child_observation(&mut self, observation: &ShapeObservation) {
        let path = CommandPath::new(observation.path.clone());
        if path.is_empty() {
            return;
        }

        self.command_mut(path)
            .apply_invalid_child(&observation.evidence_id, rejected_by_runtime(observation));
    }

    fn apply_invalid_flag_observation(&mut self, observation: &ShapeObservation) {
        let path = CommandPath::new(observation.path.clone());
        if path.is_empty() {
            return;
        }

        self.command_mut(path)
            .apply_invalid_flag(&observation.evidence_id, rejected_by_runtime(observation));
    }

    fn command_mut(&mut self, path: CommandPath) -> &mut CommandClaim {
        self.commands
            .entry(path.clone())
            .or_insert_with(|| CommandClaim::new(path))
    }
}

#[derive(Debug)]
pub struct CommandClaim {
    path: CommandPath,
    summary: Option<String>,
    belief: Belief,
    runtime_confirmed: bool,
    help_unavailable: bool,
    has_child_candidates: bool,
    invalid_child_rejected: bool,
    invalid_flag_rejected: bool,
    evidence: Vec<String>,
}

impl CommandClaim {
    fn new(path: CommandPath) -> Self {
        Self {
            path,
            summary: None,
            belief: Belief::with_prior(COMMAND_PRIOR),
            runtime_confirmed: false,
            help_unavailable: false,
            has_child_candidates: false,
            invalid_child_rejected: false,
            invalid_flag_rejected: false,
            evidence: Vec::new(),
        }
    }

    fn apply_layout_candidate(
        &mut self,
        summary: Option<String>,
        evidence_id: &str,
        evidence_detail: &str,
    ) {
        self.belief.update(1.0);
        if self.summary.is_none() {
            self.summary = summary;
        }
        self.evidence
            .push(format!("{evidence_id}:{evidence_detail}"));
    }

    fn apply_runtime_help(&mut self, evidence_id: &str, help_like: bool) {
        if help_like {
            self.belief.update(4.0);
            self.runtime_confirmed = true;
        } else {
            self.belief.update(-2.0);
            self.help_unavailable = true;
        }
        self.evidence.push(evidence_id.to_owned());
    }

    fn apply_child_candidate(&mut self, evidence_id: &str) {
        self.has_child_candidates = true;
        self.evidence
            .push(format!("{evidence_id}:child command row"));
    }

    fn apply_invalid_child(&mut self, evidence_id: &str, rejected: bool) {
        if rejected {
            self.invalid_child_rejected = true;
            self.belief.update(0.5);
        }
        self.evidence.push(evidence_id.to_owned());
    }

    fn apply_invalid_flag(&mut self, evidence_id: &str, rejected: bool) {
        if rejected {
            self.invalid_flag_rejected = true;
            self.belief.update(0.5);
        }
        self.evidence.push(evidence_id.to_owned());
    }

    pub fn path(&self) -> &CommandPath {
        &self.path
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    pub fn confidence(&self) -> f64 {
        self.belief.probability()
    }

    pub fn runtime_confirmed(&self) -> bool {
        self.runtime_confirmed
    }

    pub fn help_unavailable(&self) -> bool {
        self.help_unavailable
    }

    pub fn has_child_candidates(&self) -> bool {
        self.has_child_candidates
    }

    pub fn invalid_child_rejected(&self) -> bool {
        self.invalid_child_rejected
    }

    pub fn invalid_flag_rejected(&self) -> bool {
        self.invalid_flag_rejected
    }

    pub fn evidence(&self) -> &[String] {
        &self.evidence
    }
}

#[derive(Debug)]
pub struct FlagClaim {
    command_path: CommandPath,
    name: String,
    short: Option<String>,
    summary: Option<String>,
    belief: Belief,
    evidence: Vec<String>,
}

impl FlagClaim {
    fn new(command_path: CommandPath, name: String) -> Self {
        Self {
            command_path,
            name,
            short: None,
            summary: None,
            belief: Belief::with_prior(FLAG_PRIOR),
            evidence: Vec::new(),
        }
    }

    fn apply_layout_candidate(
        &mut self,
        short: Option<String>,
        summary: Option<String>,
        evidence_id: &str,
        evidence_detail: &str,
    ) {
        self.belief.update(1.0);
        if self.short.is_none() {
            self.short = short;
        }
        if self.summary.is_none() {
            self.summary = summary;
        }
        self.evidence
            .push(format!("{evidence_id}:{evidence_detail}"));
    }

    pub fn command_path(&self) -> &CommandPath {
        &self.command_path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn short(&self) -> Option<&str> {
        self.short.as_deref()
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    pub fn confidence(&self) -> f64 {
        self.belief.probability()
    }

    pub fn evidence(&self) -> &[String] {
        &self.evidence
    }
}

fn successful_help_like(observation: &ShapeObservation) -> bool {
    matches!(
        &observation.process.status,
        ProcessStatus::Exited { code: Some(0) }
    ) && observation
        .process
        .stdout
        .text
        .as_deref()
        .is_some_and(layout::is_help_like)
}

fn rejected_by_runtime(observation: &ShapeObservation) -> bool {
    matches!(
        &observation.process.status,
        ProcessStatus::Exited { code: Some(code) } if *code != 0
    )
}

fn absolutize_candidate_path(current_path: &[String], candidate: Vec<String>) -> Vec<String> {
    if current_path.is_empty() || candidate.starts_with(current_path) {
        return candidate;
    }

    let mut full_path = current_path.to_vec();
    full_path.extend(candidate);
    full_path
}

fn is_child_path(current_path: &[String], candidate_path: &[String]) -> bool {
    !current_path.is_empty()
        && candidate_path.len() > current_path.len()
        && candidate_path.starts_with(current_path)
}

#[cfg(test)]
mod tests {
    use super::ClaimSet;
    use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
    use crate::observation::ShapeObservation;
    use crate::process::OutputCapture;

    #[test]
    fn claims_track_layout_and_runtime_confirmation() {
        let observations = vec![
            observation(
                "e_000003",
                ProbeIntent::Help,
                vec![],
                "Commands:\n  measure  Run probes\n",
                Some(0),
            ),
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["measure".to_owned()],
                "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
                Some(0),
            ),
        ];

        let claims = ClaimSet::from_observations("cliare", &observations);
        let measure = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["measure"])
            .expect("measure claim exists");

        assert!(measure.runtime_confirmed());
        assert!(measure.confidence() > 0.90);
        assert!(claims.flags().any(|flag| flag.name() == "--out"));
    }

    #[test]
    fn claims_record_negative_diagnostic_probes() {
        let observations = vec![
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["measure".to_owned()],
                "Usage: cliare measure <TARGET>\n\nCommands:\n  nested  Nested command\n",
                Some(0),
            ),
            observation(
                "e_000007",
                ProbeIntent::InvalidChild,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
            observation(
                "e_000009",
                ProbeIntent::InvalidFlag,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
        ];

        let claims = ClaimSet::from_observations("cliare", &observations);
        let measure = claims
            .commands()
            .find(|claim| claim.path().as_slice() == ["measure"])
            .expect("measure claim exists");

        assert!(measure.invalid_child_rejected());
        assert!(measure.invalid_flag_rejected());
        assert!(measure.has_child_candidates());
    }

    fn observation(
        evidence_id: &str,
        intent: ProbeIntent,
        path: Vec<String>,
        stdout: &str,
        exit_code: Option<i32>,
    ) -> ShapeObservation {
        ShapeObservation {
            evidence_id: evidence_id.to_owned(),
            intent,
            path,
            process: ProcessCompleted {
                probe_id: "p_000001".to_owned(),
                argv: vec!["cliare".to_owned(), "--help".to_owned()],
                status: ProcessStatus::Exited { code: exit_code },
                duration_ms: 1,
                stdout: output(stdout),
                stderr: output(""),
            },
        }
    }

    fn output(text: &str) -> OutputCapture {
        OutputCapture {
            sha256: "unused".to_owned(),
            bytes: text.len(),
            retained_bytes: text.len(),
            truncated: false,
            text: Some(text.to_owned()),
        }
    }
}
