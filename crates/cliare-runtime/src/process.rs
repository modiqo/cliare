use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::time;

use crate::sandbox::{ProcessSandbox, SideEffectSummary};
use cliare_core::error::{CliareError, Result};
use cliare_core::probe_intent::ProbeIntent;

const STREAM_DRAIN_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProbeSpec {
    pub args: Vec<String>,
    pub path: Vec<String>,
    pub intent: ProbeIntent,
}

impl ProbeSpec {
    pub fn new<const N: usize>(args: [&str; N], intent: ProbeIntent) -> Self {
        Self {
            args: args.into_iter().map(str::to_owned).collect(),
            path: Vec::new(),
            intent,
        }
    }

    pub fn from_vec(args: Vec<String>, intent: ProbeIntent) -> Self {
        Self {
            args,
            path: Vec::new(),
            intent,
        }
    }

    pub fn path_help(path: Vec<String>) -> Self {
        let mut args = path.clone();
        args.push("--help".to_owned());
        Self {
            args,
            path,
            intent: ProbeIntent::Help,
        }
    }

    pub fn help_path(path: Vec<String>) -> Self {
        let mut args = Vec::with_capacity(path.len() + 1);
        args.push("help".to_owned());
        args.extend(path.iter().cloned());
        Self {
            args,
            path,
            intent: ProbeIntent::Help,
        }
    }

    pub fn invalid_child(path: Vec<String>, token: String) -> Self {
        let mut args = path.clone();
        args.push(token);
        Self {
            args,
            path,
            intent: ProbeIntent::InvalidChild,
        }
    }

    pub fn invalid_flag(path: Vec<String>, flag: String) -> Self {
        let mut args = path.clone();
        args.push(flag);
        Self {
            args,
            path,
            intent: ProbeIntent::InvalidFlag,
        }
    }

    pub fn output_mode(path: Vec<String>, argv_fragment: Vec<String>, intent: ProbeIntent) -> Self {
        let mut args = path.clone();
        args.extend(argv_fragment);
        Self { args, path, intent }
    }

    pub fn output_mode_help(
        path: Vec<String>,
        argv_fragment: Vec<String>,
        intent: ProbeIntent,
    ) -> Self {
        let mut args = path.clone();
        args.extend(argv_fragment);
        args.push("--help".to_owned());
        Self { args, path, intent }
    }

    pub fn argv(&self, target: &Path) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.args.len() + 1);
        argv.push(target.display().to_string());
        argv.extend(self.args.iter().cloned());
        argv
    }
}

#[derive(Debug, Clone)]
pub struct TargetProcess {
    target: PathBuf,
    timeout: Duration,
    output_limit_bytes: usize,
}

impl TargetProcess {
    pub fn new(target: PathBuf, timeout: Duration, output_limit_bytes: usize) -> Self {
        Self {
            target,
            timeout,
            output_limit_bytes,
        }
    }

    pub async fn run(&self, probe: &ProbeSpec, sandbox: ProcessSandbox) -> Result<ProbeOutcome> {
        let started = Instant::now();
        let argv = self.argv(probe);
        let before_side_effects = sandbox.snapshot().await?;
        let mut command = Command::new(&self.target);
        command
            .args(&probe.args)
            .current_dir(&sandbox.cwd)
            .env_clear()
            .envs(&sandbox.env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(CliareError::Spawn)?;

        let stdout = child
            .stdout
            .take()
            .ok_or(CliareError::MissingPipe { stream: "stdout" })?;
        let stderr = child
            .stderr
            .take()
            .ok_or(CliareError::MissingPipe { stream: "stderr" })?;
        let stdout_limit = self.output_limit_bytes;
        let stderr_limit = self.output_limit_bytes;

        let stdout_task = tokio::spawn(async move { read_bounded(stdout, stdout_limit).await });
        let stderr_task = tokio::spawn(async move { read_bounded(stderr, stderr_limit).await });

        let mut timed_out = false;
        let status = match time::timeout(self.timeout, child.wait()).await {
            Ok(wait_result) => Some(wait_result.map_err(CliareError::Wait)?),
            Err(_) => {
                timed_out = true;
                let _ = child.start_kill();
                let _ = child.wait().await;
                None
            }
        };

        let stdout = collect_output(stdout_task).await?;
        let stderr = collect_output(stderr_task).await?;
        let after_side_effects = sandbox.snapshot().await?;
        let side_effects = before_side_effects.diff(&after_side_effects);

        Ok(ProbeOutcome {
            argv,
            exit_code: status.and_then(|status| status.code()),
            timed_out,
            duration: started.elapsed(),
            stdout,
            stderr,
            side_effects,
        })
    }

    fn argv(&self, probe: &ProbeSpec) -> Vec<String> {
        probe.argv(&self.target)
    }
}

async fn collect_output(
    mut task: tokio::task::JoinHandle<Result<OutputCapture>>,
) -> Result<OutputCapture> {
    tokio::select! {
        result = &mut task => result.map_err(CliareError::Join)?,
        _ = time::sleep(STREAM_DRAIN_TIMEOUT) => {
            task.abort();
            Ok(OutputCapture::abandoned())
        }
    }
}

#[derive(Debug)]
pub struct ProbeOutcome {
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub duration: Duration,
    pub stdout: OutputCapture,
    pub stderr: OutputCapture,
    pub side_effects: SideEffectSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputCapture {
    pub sha256: String,
    pub bytes: usize,
    pub retained_bytes: usize,
    pub truncated: bool,
    pub text: Option<String>,
}

impl OutputCapture {
    fn abandoned() -> Self {
        Self {
            sha256: format!("{:x}", Sha256::digest([])),
            bytes: 0,
            retained_bytes: 0,
            truncated: true,
            text: None,
        }
    }
}

async fn read_bounded<R>(mut reader: R, limit: usize) -> Result<OutputCapture>
where
    R: AsyncRead + Unpin,
{
    let mut hasher = Sha256::new();
    let mut retained = Vec::with_capacity(limit.min(16 * 1024));
    let mut total = 0_usize;
    let mut truncated = false;
    let mut buffer = [0_u8; 8 * 1024];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .await
            .map_err(CliareError::ReadOutput)?;
        if bytes_read == 0 {
            break;
        }

        total = total.saturating_add(bytes_read);
        hasher.update(&buffer[..bytes_read]);

        if retained.len() < limit {
            let remaining = limit - retained.len();
            let to_copy = remaining.min(bytes_read);
            retained.extend_from_slice(&buffer[..to_copy]);
            truncated |= to_copy < bytes_read;
        } else {
            truncated = true;
        }
    }

    let text = String::from_utf8(retained.clone()).ok();

    Ok(OutputCapture {
        sha256: format!("{:x}", hasher.finalize()),
        bytes: total,
        retained_bytes: retained.len(),
        truncated,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::read_bounded;

    #[tokio::test]
    async fn bounded_reader_hashes_all_bytes_but_retains_limit() {
        let input = "abcdef".as_bytes();
        let output = read_bounded(input, 3).await.expect("read succeeds");

        assert_eq!(output.bytes, 6);
        assert_eq!(output.retained_bytes, 3);
        assert!(output.truncated);
        assert_eq!(output.text.as_deref(), Some("abc"));
    }
}
