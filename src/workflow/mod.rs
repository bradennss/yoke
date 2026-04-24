pub mod classify;
pub mod cleanup;
pub mod context;
pub mod phase;
pub mod pipeline;
pub mod plan;
pub mod review;
pub mod spec;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rand::RngExt;

use crate::claude::{ClaudeInvocation, StreamEvent};
use crate::config::{Effort, RetryConfig, YokeConfig};
use crate::intent::IntentState;
use crate::intent::store::IntentStore;
use crate::output::StreamDisplay;
use crate::prompts::PromptLoader;
use crate::state::StepCost;
use crate::template;

pub struct IntentContext<'a> {
    pub project_dir: &'a Path,
    pub work_dir: PathBuf,
    pub intent_dir: PathBuf,
    pub specs_dir: PathBuf,
    pub intent: &'a mut IntentState,
    pub config: &'a YokeConfig,
    pub store: &'a IntentStore,
    pub loader: PromptLoader,
    pub system_prompt: String,
    pub dry_run: bool,
}

impl<'a> IntentContext<'a> {
    pub fn save_intent(&self) -> Result<()> {
        self.intent.save(&self.intent_dir.join("intent.json"))
    }

    pub fn plan_path(&self) -> PathBuf {
        self.intent_dir.join("plan.md")
    }

    pub fn phases_dir(&self) -> PathBuf {
        self.intent_dir.join("phases")
    }

    pub fn research_dir(&self) -> PathBuf {
        self.intent_dir.join("research")
    }

    pub fn plans_dir(&self) -> PathBuf {
        self.intent_dir.join("plans")
    }

    pub fn handoffs_dir(&self) -> PathBuf {
        self.intent_dir.join("handoffs")
    }

    pub fn extracts_dir(&self) -> PathBuf {
        self.intent_dir.join("extracts")
    }

    pub fn product_spec_path(&self) -> PathBuf {
        self.specs_dir.join("product.md")
    }

    pub fn technical_spec_path(&self) -> PathBuf {
        self.specs_dir.join("technical.md")
    }

    pub fn accumulate_cost(&mut self, phase_idx: usize, cost: f64, step_label: &str) {
        self.intent.phases[phase_idx].cost_usd += cost;
        self.intent.phases[phase_idx].step_costs.push(StepCost {
            step: step_label.to_string(),
            cost_usd: cost,
        });
        self.intent.total_cost_usd += cost;
    }
}

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

pub fn format_gate_commands(commands: &[String]) -> String {
    if commands.is_empty() {
        return String::new();
    }
    let mut lines = vec![
        "## Gate commands".to_string(),
        String::new(),
        "All gate commands must pass before work is considered complete:".to_string(),
    ];
    for (i, cmd) in commands.iter().enumerate() {
        lines.push(format!("{}. `{cmd}`", i + 1));
    }
    lines.join("\n")
}

pub fn build_system_prompt(config: &YokeConfig, loader: &PromptLoader) -> Result<String> {
    let template = loader.load("system")?;
    let gate_commands = format_gate_commands(&config.gate_commands);

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
    use crate::config::YokeConfig;

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

    fn test_config(name: &str) -> YokeConfig {
        let toml_str = format!(
            r#"
[project]
name = "{name}"
"#
        );
        toml::from_str(&toml_str).unwrap()
    }

    #[test]
    fn build_system_prompt_with_gate_commands() {
        let loader = PromptLoader::new(None);
        let mut config = test_config("test");
        config.gate_commands = vec!["cargo fmt".to_string(), "cargo test".to_string()];

        let prompt = build_system_prompt(&config, &loader).unwrap();
        assert!(prompt.contains("Gate commands"));
        assert!(prompt.contains("`cargo fmt`"));
        assert!(prompt.contains("`cargo test`"));
        assert!(prompt.contains("test"));
    }

    #[test]
    fn build_system_prompt_no_gate_commands() {
        let loader = PromptLoader::new(None);
        let config = test_config("minimal");

        let prompt = build_system_prompt(&config, &loader).unwrap();
        assert!(!prompt.contains("Gate commands"));
        assert!(prompt.contains("minimal"));
    }

    #[test]
    fn build_system_prompt_contains_principles() {
        let loader = PromptLoader::new(None);
        let config = test_config("any");

        let prompt = build_system_prompt(&config, &loader).unwrap();
        assert!(prompt.contains("Maintainability"));
        assert!(prompt.contains("Hard rules"));
        assert!(prompt.contains("comments earn their keep"));
    }

    #[test]
    fn intent_context_path_methods() {
        use crate::intent::store::IntentStore;
        use crate::intent::{Classification, Depth, IntentState};
        use std::path::Path;

        let mut intent = IntentState::new(
            1,
            "Test build".to_string(),
            "Building test".to_string(),
            Classification::Build,
            Depth::Full,
        );
        let config = test_config("test");
        let store = IntentStore::new(Path::new("/project"));
        let loader = PromptLoader::new(None);

        let mut ctx = IntentContext {
            project_dir: Path::new("/project"),
            work_dir: PathBuf::from("/project"),
            intent_dir: PathBuf::from("/project/.yoke/intents/i-001-test-build"),
            specs_dir: PathBuf::from("/project/.yoke/specs"),
            intent: &mut intent,
            config: &config,
            store: &store,
            loader,
            system_prompt: String::new(),
            dry_run: false,
        };

        assert_eq!(
            ctx.plan_path(),
            PathBuf::from("/project/.yoke/intents/i-001-test-build/plan.md")
        );
        assert_eq!(
            ctx.phases_dir(),
            PathBuf::from("/project/.yoke/intents/i-001-test-build/phases")
        );
        assert_eq!(
            ctx.research_dir(),
            PathBuf::from("/project/.yoke/intents/i-001-test-build/research")
        );
        assert_eq!(
            ctx.plans_dir(),
            PathBuf::from("/project/.yoke/intents/i-001-test-build/plans")
        );
        assert_eq!(
            ctx.handoffs_dir(),
            PathBuf::from("/project/.yoke/intents/i-001-test-build/handoffs")
        );
        assert_eq!(
            ctx.extracts_dir(),
            PathBuf::from("/project/.yoke/intents/i-001-test-build/extracts")
        );
        assert_eq!(
            ctx.product_spec_path(),
            PathBuf::from("/project/.yoke/specs/product.md")
        );
        assert_eq!(
            ctx.technical_spec_path(),
            PathBuf::from("/project/.yoke/specs/technical.md")
        );

        ctx.intent.phases.push(crate::state::PhaseState {
            number: 1,
            title: "Setup".to_string(),
            status: crate::state::PhaseStatus::InProgress,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: None,
        });
        ctx.accumulate_cost(0, 0.25, "research");
        assert_eq!(ctx.intent.phases[0].cost_usd, 0.25);
        assert_eq!(ctx.intent.phases[0].step_costs.len(), 1);
        assert_eq!(ctx.intent.total_cost_usd, 0.25);
    }
}
