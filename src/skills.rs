use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tokio::fs;

use crate::cli::{
    SkillAgent, SkillInstallScope, SkillsArgs, SkillsCommand, SkillsInstallArgs, SkillsListArgs,
    SkillsListFormat,
};
use crate::error::{CliareError, Result};

const SKILL_NAME: &str = "cliare-artifact-review";
const SKILL_MD: &str = include_str!("../skills/cliare-artifact-review/SKILL.md");
const CURSOR_RULE: &str = include_str!("../skills/cursor/cliare-artifact-review.mdc");

const PERSONAS: &[PersonaCommand] = &[
    PersonaCommand {
        label: "maintainer",
        role: "CLI maintainer",
        emphasis: "implementation defects, CLI contract gaps, fixture needs, and verification commands",
    },
    PersonaCommand {
        label: "harness",
        role: "agent harness builder",
        emphasis: "commands ready for routing, commands to hold, output contracts, side effects, and policy gates",
    },
    PersonaCommand {
        label: "platform",
        role: "platform engineer",
        emphasis: "CI gates, guard thresholds, baseline drift, policy exceptions, and rollout actions",
    },
    PersonaCommand {
        label: "security",
        role: "security reviewer",
        emphasis: "side effects, credential-like paths, auth/profile gates, preconditions, and approval constraints",
    },
    PersonaCommand {
        label: "oss",
        role: "open-source maintainer",
        emphasis: "publishable claims, caveats, reproducibility, score posture, and public remediation roadmap",
    },
    PersonaCommand {
        label: "devrel",
        role: "developer relations reviewer",
        emphasis: "public guidance, examples, score explanations, and evidence-backed roadmap language",
    },
    PersonaCommand {
        label: "research",
        role: "benchmark researcher",
        emphasis: "labels, evidence ids, score model caveats, calibration readiness, and replay metadata",
    },
];

#[derive(Debug, Clone)]
pub struct SkillsSummary {
    stdout: String,
}

impl SkillsSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.stdout
    }
}

pub async fn skills(args: SkillsArgs) -> Result<SkillsSummary> {
    match args.command {
        SkillsCommand::List(args) => list(args).await,
        SkillsCommand::Install(args) => install(args).await,
    }
}

async fn list(args: SkillsListArgs) -> Result<SkillsSummary> {
    let integrations = catalog();
    let stdout = match args.format {
        SkillsListFormat::Text => render_catalog_text(&integrations),
        SkillsListFormat::Json => {
            let value = serde_json::to_string_pretty(&integrations)
                .map_err(CliareError::SerializeSkillCatalog)?;
            format!("{value}\n")
        }
    };
    Ok(SkillsSummary { stdout })
}

async fn install(args: SkillsInstallArgs) -> Result<SkillsSummary> {
    let plan = install_plan(&args)?;
    let mut results = Vec::with_capacity(plan.len());
    for artifact in plan {
        let action =
            write_artifact(&artifact.path, artifact.contents.as_bytes(), args.dry_run).await?;
        results.push(InstallResult {
            agent: artifact.agent,
            kind: artifact.kind,
            path: artifact.path,
            action,
        });
    }
    Ok(SkillsSummary {
        stdout: render_install_summary(args.agent, args.scope, args.dry_run, &results),
    })
}

fn install_plan(args: &SkillsInstallArgs) -> Result<Vec<InstallArtifact>> {
    let roots = InstallRoots::resolve(args)?;
    let mut artifacts = Vec::new();

    for agent in expanded_agents(args.agent) {
        match agent {
            SkillAgent::Claude => add_claude_artifacts(&roots, &mut artifacts),
            SkillAgent::Codex => add_codex_artifacts(&roots, &mut artifacts),
            SkillAgent::Cursor => add_cursor_artifacts(&roots, &mut artifacts),
            SkillAgent::All => unreachable!("expanded_agents never returns all"),
        }
    }

    Ok(artifacts)
}

fn add_claude_artifacts(roots: &InstallRoots, artifacts: &mut Vec<InstallArtifact>) {
    let root = roots.root_for(SkillAgent::Claude);
    artifacts.push(InstallArtifact::new(
        SkillAgent::Claude,
        InstallKind::Skill,
        root.join("skills").join(SKILL_NAME).join("SKILL.md"),
        SKILL_MD.to_owned(),
    ));

    for persona in PERSONAS {
        artifacts.push(InstallArtifact::new(
            SkillAgent::Claude,
            InstallKind::Command,
            root.join("commands")
                .join(format!("cliare-{}.md", persona.label)),
            claude_persona_command(persona),
        ));
    }
}

fn add_codex_artifacts(roots: &InstallRoots, artifacts: &mut Vec<InstallArtifact>) {
    let root = roots.root_for(SkillAgent::Codex);
    artifacts.push(InstallArtifact::new(
        SkillAgent::Codex,
        InstallKind::Skill,
        root.join("skills").join(SKILL_NAME).join("SKILL.md"),
        SKILL_MD.to_owned(),
    ));
}

fn add_cursor_artifacts(roots: &InstallRoots, artifacts: &mut Vec<InstallArtifact>) {
    let root = roots.root_for(SkillAgent::Cursor);
    artifacts.push(InstallArtifact::new(
        SkillAgent::Cursor,
        InstallKind::Rule,
        root.join("rules").join("cliare-artifact-review.mdc"),
        CURSOR_RULE.to_owned(),
    ));
}

async fn write_artifact(path: &Path, contents: &[u8], dry_run: bool) -> Result<InstallAction> {
    if dry_run {
        return Ok(InstallAction::WouldWrite);
    }

    let existing = match fs::read(path).await {
        Ok(bytes) => Some(bytes),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(CliareError::ReadInstalledSkill {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if existing.as_deref() == Some(contents) {
        return Ok(InstallAction::Unchanged);
    }

    let Some(parent) = path.parent() else {
        return Err(CliareError::CreateSkillDir {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "skill artifact path has no parent directory",
            ),
        });
    };
    fs::create_dir_all(parent)
        .await
        .map_err(|source| CliareError::CreateSkillDir {
            path: parent.to_path_buf(),
            source,
        })?;

    let temp_path = atomic_temp_path(path);
    fs::write(&temp_path, contents)
        .await
        .map_err(|source| CliareError::WriteInstalledSkill {
            path: temp_path.clone(),
            source,
        })?;
    fs::rename(&temp_path, path)
        .await
        .map_err(|source| CliareError::WriteInstalledSkill {
            path: path.to_path_buf(),
            source,
        })?;

    Ok(if existing.is_some() {
        InstallAction::Updated
    } else {
        InstallAction::Created
    })
}

fn atomic_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cliare-skill");
    path.with_file_name(format!("{file_name}.tmp.{}", std::process::id()))
}

fn expanded_agents(agent: SkillAgent) -> Vec<SkillAgent> {
    match agent {
        SkillAgent::All => vec![SkillAgent::Claude, SkillAgent::Codex, SkillAgent::Cursor],
        other => vec![other],
    }
}

fn claude_persona_command(persona: &PersonaCommand) -> String {
    format!(
        r#"Use the `cliare-artifact-review` skill.

Persona: {label}
Project or target: $ARGUMENTS

Resolve the CLIARE artifact directory for the project or target. Prefer, in order:

1. An explicit directory in `$ARGUMENTS` that contains `scorecard.json`.
2. `.cliare` under the current project when it contains `scorecard.json`.
3. `.cliare/<target>` under the current project when it contains `scorecard.json`.
4. A recently generated `/tmp/cliare-*<target>*` directory that contains `scorecard.json`.

If no artifact directory exists, say that a CLIARE measurement is required and show the exact command to run. Do not invent findings.

Operational rules:

- Prefer reading `persona-{label}.md`, `persona-{label}.json`, and `issues.json` directly for persona posture.
- Use `command-index.json` or `command-index.md` for command suitability, parameters, preconditions, output contracts, and evidence pointers.
- Start with the persona table. Ask which priority row to drill into unless the user already named an issue.
- If shell is needed, use `jq` with explicit artifact file paths.
- Do not use Python, `cd`, shell redirection, heredocs, or compound shell commands for routine artifact inspection.
- Do not run exploratory scripts to discover JSON keys. Use the known CLIARE schema from the skill.
- For large affected-command lists, count by runtime state first. If the user explicitly asks for all commands, list them compactly from `issues.json` only. Do not infer false positives, root causes, or design intent from command names.

Run or refresh the persona packet when needed:

```sh
cliare report {label} --out <artifact-dir> --write
```

Read `persona-{label}.md`, `persona-{label}.json`, `issues.json`, `command-index.json`, `command-index.md`, `shape.json`, and `evidence.jsonl` when evidence is needed.

Answer as a {role}. Focus on {emphasis}.

Use this output shape:

| Priority | Meaning | Disposition | Affected | Issue | Action |
|---:|---|---|---:|---|---|

Then provide drill-down only for the selected priority row.

Do not dump raw JSON. Treat blocked, fixture-required, incomplete, and inferred commands as distinct states.
"#,
        label = persona.label,
        role = persona.role,
        emphasis = persona.emphasis
    )
}

fn render_catalog_text(integrations: &[SkillIntegration]) -> String {
    let mut text = String::new();
    writeln!(&mut text, "CLIARE installable skills").expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "| Agent | Installed artifacts | Default location |"
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text, "|---|---|---|").expect("writing to string cannot fail");
    for integration in integrations {
        writeln!(
            &mut text,
            "| `{}` | {} | `{}` |",
            integration.agent, integration.artifacts, integration.default_location
        )
        .expect("writing to string cannot fail");
    }
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "Install with `cliare skills install --agent <agent>` or `cliare skills install --agent all`."
    )
    .expect("writing to string cannot fail");
    text
}

fn render_install_summary(
    agent: SkillAgent,
    scope: SkillInstallScope,
    dry_run: bool,
    results: &[InstallResult],
) -> String {
    let mut text = String::new();
    writeln!(&mut text, "CLIARE skills install").expect("writing to string cannot fail");
    writeln!(&mut text, "agent: {}", agent.label()).expect("writing to string cannot fail");
    writeln!(&mut text, "scope: {}", scope.label()).expect("writing to string cannot fail");
    writeln!(&mut text, "dry run: {}", dry_run).expect("writing to string cannot fail");
    writeln!(&mut text, "artifacts: {}", results.len()).expect("writing to string cannot fail");
    for result in results {
        writeln!(
            &mut text,
            "- {} {} {}: {}",
            result.agent.label(),
            result.kind.label(),
            result.action.label(),
            result.path.display()
        )
        .expect("writing to string cannot fail");
    }
    text
}

fn catalog() -> Vec<SkillIntegration> {
    vec![
        SkillIntegration {
            agent: "claude",
            artifacts: "shared artifact-review skill plus /cliare-<persona> commands",
            default_location: "~/.claude/skills and ~/.claude/commands",
        },
        SkillIntegration {
            agent: "codex",
            artifacts: "shared artifact-review skill",
            default_location: "~/.codex/skills",
        },
        SkillIntegration {
            agent: "cursor",
            artifacts: "CLIARE artifact-review rule",
            default_location: "~/.cursor/rules",
        },
    ]
}

#[derive(Debug, Serialize)]
struct SkillIntegration {
    agent: &'static str,
    artifacts: &'static str,
    default_location: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct PersonaCommand {
    label: &'static str,
    role: &'static str,
    emphasis: &'static str,
}

#[derive(Debug)]
struct InstallRoots {
    scope: SkillInstallScope,
    user_home: PathBuf,
    project_dir: PathBuf,
}

impl InstallRoots {
    fn resolve(args: &SkillsInstallArgs) -> Result<Self> {
        let user_home = match args.scope {
            SkillInstallScope::User => resolve_user_home(args.home.as_deref())?,
            SkillInstallScope::Project => args.home.clone().unwrap_or_default(),
        };
        let project_dir = match args.scope {
            SkillInstallScope::User => args.project_dir.clone().unwrap_or_default(),
            SkillInstallScope::Project => resolve_project_dir(args.project_dir.as_deref())?,
        };
        Ok(Self {
            scope: args.scope,
            user_home,
            project_dir,
        })
    }

    fn root_for(&self, agent: SkillAgent) -> PathBuf {
        let directory = match agent {
            SkillAgent::Claude => ".claude",
            SkillAgent::Codex => ".codex",
            SkillAgent::Cursor => ".cursor",
            SkillAgent::All => unreachable!("expanded_agents never returns all"),
        };
        match self.scope {
            SkillInstallScope::User => self.user_home.join(directory),
            SkillInstallScope::Project => self.project_dir.join(directory),
        }
    }
}

fn resolve_user_home(override_home: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_home {
        return Ok(path.to_path_buf());
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or(CliareError::HomeDirectoryUnavailable)
}

fn resolve_project_dir(override_project_dir: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_project_dir {
        return Ok(path.to_path_buf());
    }
    env::current_dir().map_err(CliareError::CurrentDirectory)
}

#[derive(Debug)]
struct InstallArtifact {
    agent: SkillAgent,
    kind: InstallKind,
    path: PathBuf,
    contents: String,
}

impl InstallArtifact {
    fn new(agent: SkillAgent, kind: InstallKind, path: PathBuf, contents: String) -> Self {
        Self {
            agent,
            kind,
            path,
            contents,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum InstallKind {
    Skill,
    Command,
    Rule,
}

impl InstallKind {
    fn label(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Command => "command",
            Self::Rule => "rule",
        }
    }
}

#[derive(Debug)]
struct InstallResult {
    agent: SkillAgent,
    kind: InstallKind,
    path: PathBuf,
    action: InstallAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallAction {
    Created,
    Updated,
    Unchanged,
    WouldWrite,
}

impl InstallAction {
    fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Unchanged => "unchanged",
            Self::WouldWrite => "would write",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{SkillAgent, SkillInstallScope};

    #[test]
    fn all_agent_plan_includes_claude_codex_and_cursor_artifacts() {
        let args = SkillsInstallArgs {
            agent: SkillAgent::All,
            scope: SkillInstallScope::User,
            home: Some(PathBuf::from("/tmp/cliare-home")),
            project_dir: None,
            dry_run: true,
        };

        let plan = install_plan(&args).expect("valid plan");
        assert_eq!(plan.len(), 10);
        assert!(plan.iter().any(|artifact| {
            artifact
                .path
                .ends_with(".claude/commands/cliare-harness.md")
        }));
        assert!(plan.iter().any(|artifact| {
            artifact
                .path
                .ends_with(".codex/skills/cliare-artifact-review/SKILL.md")
        }));
        assert!(plan.iter().any(|artifact| {
            artifact
                .path
                .ends_with(".cursor/rules/cliare-artifact-review.mdc")
        }));
    }

    #[test]
    fn project_scope_plan_uses_project_directory_without_home() {
        let args = SkillsInstallArgs {
            agent: SkillAgent::Cursor,
            scope: SkillInstallScope::Project,
            home: None,
            project_dir: Some(PathBuf::from("/tmp/cliare-project")),
            dry_run: true,
        };

        let plan = install_plan(&args).expect("valid project plan");
        assert_eq!(plan.len(), 1);
        assert_eq!(
            plan[0].path,
            PathBuf::from("/tmp/cliare-project/.cursor/rules/cliare-artifact-review.mdc")
        );
    }

    #[tokio::test]
    async fn install_writes_and_then_reports_unchanged() {
        let home =
            env::temp_dir().join(format!("cliare-skill-install-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);

        let args = SkillsInstallArgs {
            agent: SkillAgent::Codex,
            scope: SkillInstallScope::User,
            home: Some(home.clone()),
            project_dir: None,
            dry_run: false,
        };

        let first = install(args).await.expect("first install succeeds");
        assert!(first.terminal_summary().contains("codex skill created"));
        let installed = home
            .join(".codex")
            .join("skills")
            .join(SKILL_NAME)
            .join("SKILL.md");
        assert!(installed.exists());

        let second = install(SkillsInstallArgs {
            agent: SkillAgent::Codex,
            scope: SkillInstallScope::User,
            home: Some(home.clone()),
            project_dir: None,
            dry_run: false,
        })
        .await
        .expect("second install succeeds");
        assert!(second.terminal_summary().contains("codex skill unchanged"));

        let _ = std::fs::remove_dir_all(home);
    }
}
