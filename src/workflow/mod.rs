pub mod cleanup;
pub mod context;
pub mod phase;
pub mod plan;
pub mod review;
pub mod spec;

use std::path::Path;

use anyhow::{Context, Result, bail};
use rand::RngExt;

use crate::claude::{ClaudeInvocation, StreamEvent};
use crate::config::{Effort, RetryConfig, YokeConfig};
use crate::output::StreamDisplay;
use crate::prompts::PromptLoader;
use crate::template;

pub struct SubAgentResult {
    pub result_text: String,
    pub cost_usd: f64,
    pub is_error: bool,
}

#[allow(clippy::too_many_arguments)]
pub async fn invoke_sub_agent(
    prompt: &str,
    model: &str,
    effort: Effort,
    tools: Option<&str>,
    system_prompt: Option<&str>,
    cwd: Option<&Path>,
    display: &mut StreamDisplay,
    retry_config: &RetryConfig,
    dry_run: bool,
) -> Result<SubAgentResult> {
    if dry_run {
        let display_prompt: String = prompt.chars().take(500).collect();
        eprintln!("[dry run] model: {model}, effort: {}", effort.as_str());
        if let Some(t) = tools {
            eprintln!("[dry run] tools: {t}");
        }
        eprintln!("[dry run] prompt: {display_prompt}");
        if prompt.len() > 500 {
            eprintln!("[dry run] ... ({} chars total)", prompt.len());
        }
        return Ok(SubAgentResult {
            result_text: String::new(),
            cost_usd: 0.0,
            is_error: false,
        });
    }

    let mut attempt = 0u8;

    loop {
        let mut invocation = ClaudeInvocation::new(prompt).model(model).effort(effort);

        if let Some(t) = tools {
            invocation = invocation.tools(t);
        }
        if let Some(sp) = system_prompt {
            invocation = invocation.append_system_prompt(sp);
        }
        if let Some(dir) = cwd {
            invocation = invocation.cwd(dir);
        }

        let mut process = invocation.spawn().context("spawning sub agent process")?;

        let mut result_text = None;
        let mut cost_usd = 0.0;
        let mut is_error = false;

        while let Some(event) = process.next_event().await {
            display.handle_event(&event);
            if let StreamEvent::Completion {
                result,
                total_cost_usd,
                is_error: err,
                ..
            } = event
            {
                result_text = Some(result);
                cost_usd = total_cost_usd;
                is_error = err;
            }
        }

        let _process_result = process.finish().await;

        match result_text {
            Some(text) if is_error && is_retryable(&text) => {
                if attempt >= retry_config.max_retries {
                    bail!(
                        "sub agent failed after {} retries: {text}",
                        retry_config.max_retries
                    );
                }
                let delay = backoff_delay(retry_config.base_delay_secs, attempt);
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                attempt += 1;
            }
            Some(text) => {
                return Ok(SubAgentResult {
                    result_text: text,
                    cost_usd,
                    is_error,
                });
            }
            None => {
                if attempt >= retry_config.max_retries {
                    bail!(
                        "sub agent process exited without a result event after {} retries",
                        retry_config.max_retries
                    );
                }
                let delay = backoff_delay(retry_config.base_delay_secs, attempt);
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                attempt += 1;
            }
        }
    }
}

fn is_retryable(error_text: &str) -> bool {
    let lower = error_text.to_lowercase();
    let retryable_patterns = ["rate limit", "overloaded", "server error"];

    if retryable_patterns.iter().any(|p| lower.contains(p)) {
        return true;
    }

    let non_retryable_patterns = [
        "auth",
        "invalid request",
        "permission denied",
        "not logged in",
    ];
    if non_retryable_patterns.iter().any(|p| lower.contains(p)) {
        return false;
    }

    true
}

fn backoff_delay(base_delay_secs: u64, attempt: u8) -> u64 {
    let base = base_delay_secs.saturating_mul(1u64 << attempt);
    let jitter_range = base / 4;
    if jitter_range == 0 {
        return base;
    }
    let jitter = rand::rng().random_range(0..=(jitter_range * 2));
    base.saturating_sub(jitter_range).saturating_add(jitter)
}

pub fn prompt_loader(project_dir: &Path) -> PromptLoader {
    let override_dir = project_dir.join(".yoke/prompts");
    let dir = if override_dir.exists() {
        Some(override_dir)
    } else {
        None
    };
    PromptLoader::new(dir)
}

pub fn build_system_prompt(config: &YokeConfig, loader: &PromptLoader) -> Result<String> {
    let template = loader.load("system")?;

    let gate_commands = if config.gate_commands.is_empty() {
        String::new()
    } else {
        let mut lines = vec![
            "## Gate commands".to_string(),
            String::new(),
            "All gate commands must pass before work is considered complete:".to_string(),
        ];
        for (i, cmd) in config.gate_commands.iter().enumerate() {
            lines.push(format!("{}. `{cmd}`", i + 1));
        }
        lines.join("\n")
    };

    Ok(template::replace_vars(
        &template,
        &[
            ("project_name", &config.project.name),
            ("gate_commands", &gate_commands),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ModelConfig, ProjectConfig, YokeConfig};

    #[test]
    fn is_retryable_rate_limit() {
        assert!(is_retryable("rate limit exceeded"));
        assert!(is_retryable("Rate Limit hit, please slow down"));
    }

    #[test]
    fn is_retryable_overloaded() {
        assert!(is_retryable("server is overloaded"));
        assert!(is_retryable("API overloaded, try again"));
    }

    #[test]
    fn is_retryable_server_error() {
        assert!(is_retryable("internal server error"));
        assert!(is_retryable("Server Error 500"));
    }

    #[test]
    fn not_retryable_auth() {
        assert!(!is_retryable("authentication failed"));
        assert!(!is_retryable("Auth token expired"));
    }

    #[test]
    fn not_retryable_invalid_request() {
        assert!(!is_retryable("invalid request format"));
    }

    #[test]
    fn not_retryable_not_logged_in() {
        assert!(!is_retryable("Not logged in"));
    }

    #[test]
    fn not_retryable_permission_denied() {
        assert!(!is_retryable("permission denied for this resource"));
    }

    #[test]
    fn unknown_error_is_retryable() {
        assert!(is_retryable("something unexpected happened"));
    }

    #[test]
    fn backoff_delay_increases_with_attempts() {
        let d0 = backoff_delay(10, 0);
        let d1 = backoff_delay(10, 1);
        let d2 = backoff_delay(10, 2);

        assert!((8..=12).contains(&d0), "attempt 0 delay was {d0}");
        assert!((15..=25).contains(&d1), "attempt 1 delay was {d1}");
        assert!((30..=50).contains(&d2), "attempt 2 delay was {d2}");
    }

    #[test]
    fn backoff_delay_zero_base() {
        let d = backoff_delay(0, 0);
        assert_eq!(d, 0);
    }

    #[test]
    fn backoff_delay_no_overflow() {
        let d = backoff_delay(u64::MAX, 10);
        assert!(d > 0);
    }

    #[test]
    fn build_system_prompt_with_gate_commands() {
        let loader = PromptLoader::new(None);
        let config = YokeConfig {
            project: ProjectConfig {
                name: "test".to_string(),
            },
            models: ModelConfig::default(),
            effort: Default::default(),
            gate_commands: vec!["cargo fmt".to_string(), "cargo test".to_string()],
            interaction: Default::default(),
            git: Default::default(),
            review: Default::default(),
            retry: Default::default(),
            context: Default::default(),
        };

        let prompt = build_system_prompt(&config, &loader).unwrap();
        assert!(prompt.contains("Gate commands"));
        assert!(prompt.contains("`cargo fmt`"));
        assert!(prompt.contains("`cargo test`"));
        assert!(prompt.contains("test"));
    }

    #[test]
    fn build_system_prompt_no_gate_commands() {
        let loader = PromptLoader::new(None);
        let config = YokeConfig {
            project: ProjectConfig {
                name: "minimal".to_string(),
            },
            models: ModelConfig::default(),
            effort: Default::default(),
            gate_commands: vec![],
            interaction: Default::default(),
            git: Default::default(),
            review: Default::default(),
            retry: Default::default(),
            context: Default::default(),
        };

        let prompt = build_system_prompt(&config, &loader).unwrap();
        assert!(!prompt.contains("Gate commands"));
        assert!(prompt.contains("minimal"));
    }

    #[test]
    fn build_system_prompt_contains_principles() {
        let loader = PromptLoader::new(None);
        let config = YokeConfig {
            project: ProjectConfig {
                name: "any".to_string(),
            },
            models: ModelConfig::default(),
            effort: Default::default(),
            gate_commands: vec![],
            interaction: Default::default(),
            git: Default::default(),
            review: Default::default(),
            retry: Default::default(),
            context: Default::default(),
        };

        let prompt = build_system_prompt(&config, &loader).unwrap();
        assert!(prompt.contains("Maintainability"));
        assert!(prompt.contains("Hard rules"));
        assert!(prompt.contains("comments earn their keep"));
    }
}
