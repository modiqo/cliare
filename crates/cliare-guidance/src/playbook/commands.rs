use std::path::{Path, PathBuf};

use cliare_cli::cli::PlaybookArgs;
use cliare_report::report_format::shell_arg;

use super::{DEFAULT_OUT_PLACEHOLDER, ISSUE_PLACEHOLDER, TARGET_PLACEHOLDER};

#[derive(Debug)]
pub(super) struct CommandBuilder<'a> {
    target: &'a str,
    out: &'a Path,
    context: Option<&'a str>,
}

impl<'a> CommandBuilder<'a> {
    pub(super) fn new(target: &'a str, out: &'a Path, context: Option<&'a str>) -> Self {
        Self {
            target,
            out,
            context,
        }
    }

    pub(super) fn measure(&self, profile: &str) -> String {
        format!(
            "cliare measure {} --out {} --profile {} --refresh",
            shell_token(self.target),
            shell_path(self.out),
            profile
        )
    }

    pub(super) fn large_measure(&self) -> String {
        format!(
            "cliare measure {} --out {} --profile deep --max-depth 12 --max-probes 5000 --concurrency 8 --refresh",
            shell_token(self.target),
            shell_path(self.out)
        )
    }

    pub(super) fn detached_measure(&self) -> String {
        format!("{} --detach", self.large_measure())
    }

    pub(super) fn authenticated_measure(&self) -> String {
        format!(
            "cliare measure {} --out {} --context authenticated --auth-state present --execution-mode host --profile deep --refresh",
            shell_token(self.target),
            shell_path(self.out)
        )
    }

    pub(super) fn job_status(&self) -> String {
        let mut command = format!("cliare jobs status --out {}", shell_path(self.out));
        self.push_context(&mut command);
        command
    }

    pub(super) fn report(&self, persona: &str, extra: &[&str]) -> String {
        let mut command = format!("cliare report {} --out {}", persona, shell_path(self.out));
        self.push_context(&mut command);
        for arg in extra {
            command.push(' ');
            command.push_str(arg);
        }
        command
    }

    pub(super) fn describe(&self, extra: &[&str]) -> String {
        let mut command = format!("cliare describe {}", shell_path(self.out));
        self.push_context(&mut command);
        for arg in extra {
            command.push(' ');
            command.push_str(arg);
        }
        command
    }

    pub(super) fn issues_list(&self, format: &str) -> String {
        let mut command = format!("cliare issues list --out {}", shell_path(self.out));
        self.push_context(&mut command);
        command.push_str(" --format ");
        command.push_str(format);
        command
    }

    pub(super) fn surface_query(&self, intent: &str, extra: &[&str]) -> String {
        let mut command = format!(
            "cliare surface query {} --out {}",
            shell_arg(intent),
            shell_path(self.out)
        );
        self.push_context(&mut command);
        for arg in extra {
            command.push(' ');
            command.push_str(arg);
        }
        command
    }

    pub(super) fn surface_explain(&self, command_path: &str, extra: &[&str]) -> String {
        let mut command = format!(
            "cliare surface explain {} --out {}",
            shell_arg(command_path),
            shell_path(self.out)
        );
        self.push_context(&mut command);
        for arg in extra {
            command.push(' ');
            command.push_str(arg);
        }
        command
    }

    pub(super) fn surface_list(&self, extra: &[&str]) -> String {
        let mut command = format!("cliare surface list --out {}", shell_path(self.out));
        self.push_context(&mut command);
        for arg in extra {
            command.push(' ');
            command.push_str(arg);
        }
        command
    }

    pub(super) fn skills_install(&self) -> String {
        "cliare skills install --agent all --scope project".to_owned()
    }

    pub(super) fn metadata_json(&self) -> String {
        "cliare metadata --format json".to_owned()
    }

    pub(super) fn issues_mark(&self, status: &str, reason: &str) -> String {
        let mut command = format!(
            "cliare issues mark {} --out {}",
            ISSUE_PLACEHOLDER,
            shell_path(self.out)
        );
        self.push_context(&mut command);
        command.push_str(" --status ");
        command.push_str(status);
        command.push_str(" --reason ");
        command.push_str(&shell_arg(reason));
        command
    }

    pub(super) fn guard(&self) -> String {
        format!(
            "cliare guard {} --baseline {} --out {} --profile deep --allowed-drop 2",
            shell_token(self.target),
            shell_path(&baseline_scorecard_path(self.target)),
            shell_path(self.out)
        )
    }

    fn push_context(&self, command: &mut String) {
        if let Some(context) = self.context {
            command.push_str(" --context ");
            command.push_str(&shell_arg(context));
        }
    }
}

pub(super) fn shell_path(path: &Path) -> String {
    shell_arg(&path.display().to_string())
}

pub(super) fn shell_token(value: &str) -> String {
    if value == TARGET_PLACEHOLDER {
        value.to_owned()
    } else {
        shell_arg(value)
    }
}

pub(super) fn effective_artifact_dir(args: &PlaybookArgs, target: &str) -> PathBuf {
    if args.out == Path::new(DEFAULT_OUT_PLACEHOLDER) {
        if target == TARGET_PLACEHOLDER {
            PathBuf::from(DEFAULT_OUT_PLACEHOLDER)
        } else {
            PathBuf::from(".cliare").join(artifact_dir_segment(target))
        }
    } else {
        args.out.clone()
    }
}

pub(super) fn artifact_dir_segment(target: &str) -> String {
    let raw = Path::new(target)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(target);
    let mut segment = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            segment.push(ch);
        } else if !segment.ends_with('-') {
            segment.push('-');
        }
    }
    let segment = segment.trim_matches('-');
    if segment.is_empty() {
        "target-cli".to_owned()
    } else {
        segment.to_owned()
    }
}

pub(super) fn baseline_scorecard_path(target: &str) -> PathBuf {
    if target == TARGET_PLACEHOLDER {
        PathBuf::from(".cliare-baseline")
            .join(TARGET_PLACEHOLDER)
            .join("scorecard.json")
    } else {
        PathBuf::from(".cliare-baseline")
            .join(artifact_dir_segment(target))
            .join("scorecard.json")
    }
}
