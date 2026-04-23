use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::io::BufReader;
use tokio::process::{Child, ChildStdout};
use tokio::task::JoinHandle;

use crate::config::Effort;

use super::stream::EventStream;
use super::types::StreamEvent;

pub struct ClaudeInvocation {
    prompt: String,
    model: Option<String>,
    effort: Option<Effort>,
    tools: Option<String>,
    append_system_prompt: Option<String>,
    max_budget_usd: Option<f64>,
    cwd: Option<PathBuf>,
}

impl ClaudeInvocation {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: None,
            effort: None,
            tools: None,
            append_system_prompt: None,
            max_budget_usd: None,
            cwd: None,
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn effort(mut self, effort: Effort) -> Self {
        self.effort = Some(effort);
        self
    }

    pub fn tools(mut self, tools: impl Into<String>) -> Self {
        self.tools = Some(tools.into());
        self
    }

    pub fn append_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.append_system_prompt = Some(prompt.into());
        self
    }

    pub fn max_budget_usd(mut self, budget: f64) -> Self {
        self.max_budget_usd = Some(budget);
        self
    }

    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    pub fn spawn(self) -> Result<ClaudeProcess> {
        let mut cmd = tokio::process::Command::new("claude");

        cmd.arg("-p").arg(&self.prompt);
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--verbose");
        cmd.arg("--include-partial-messages");
        cmd.arg("--dangerously-skip-permissions");
        cmd.arg("--no-session-persistence");
        let effort_value = self.effort.unwrap_or_default();
        cmd.arg("--effort").arg(effort_value.as_str());

        if let Some(ref model) = self.model {
            cmd.arg("--model").arg(model);
        }
        if let Some(ref tools) = self.tools {
            cmd.arg("--tools").arg(tools);
        }
        if let Some(ref sys_prompt) = self.append_system_prompt {
            cmd.arg("--append-system-prompt").arg(sys_prompt);
        }
        if let Some(budget) = self.max_budget_usd {
            cmd.arg("--max-budget-usd").arg(budget.to_string());
        }
        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        let mut child = cmd
            .spawn()
            .context("failed to spawn claude process; is the claude CLI installed?")?;

        let stdout = child
            .stdout
            .take()
            .context("failed to capture claude stdout")?;
        let stderr = child
            .stderr
            .take()
            .context("failed to capture claude stderr")?;

        let stderr_handle = tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut buf = String::new();
            let mut reader = stderr;
            let _ = reader.read_to_string(&mut buf).await;
            buf
        });

        let events = EventStream::new(BufReader::new(stdout));

        Ok(ClaudeProcess {
            child,
            events,
            stderr_handle,
        })
    }
}

pub struct ClaudeProcess {
    child: Child,
    events: EventStream<BufReader<ChildStdout>>,
    stderr_handle: JoinHandle<String>,
}

impl ClaudeProcess {
    pub async fn next_event(&mut self) -> Option<StreamEvent> {
        self.events.next_event().await
    }

    pub async fn finish(mut self) -> Result<ProcessResult> {
        let status = self
            .child
            .wait()
            .await
            .context("failed to wait for claude process")?;

        let stderr = self.stderr_handle.await.unwrap_or_default();
        if !stderr.is_empty() {
            for line in stderr.lines() {
                eprintln!("claude stderr: {line}");
            }
        }

        Ok(ProcessResult {
            exit_code: status.code(),
        })
    }
}

pub struct ProcessResult {
    pub exit_code: Option<i32>,
}
