use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct YokeConfig {
    pub project: ProjectConfig,
    #[serde(default)]
    pub models: ModelConfig,
    #[serde(default)]
    pub effort: EffortConfig,
    #[serde(default)]
    pub gate_commands: Vec<String>,
    #[serde(default)]
    pub interaction: InteractionMode,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub review: ReviewConfig,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(default)]
    pub context: ContextConfig,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    #[serde(default = "default_opus")]
    pub spec: String,
    #[serde(default = "default_sonnet")]
    pub research: String,
    #[serde(default = "default_opus")]
    pub planning: String,
    #[serde(default = "default_sonnet")]
    pub review: String,
    #[serde(default = "default_opus")]
    pub execution: String,
    #[serde(default = "default_sonnet")]
    pub code_review: String,
    #[serde(default = "default_haiku")]
    pub spec_extract: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            spec: default_opus(),
            research: default_sonnet(),
            planning: default_opus(),
            review: default_sonnet(),
            execution: default_opus(),
            code_review: default_sonnet(),
            spec_extract: default_haiku(),
        }
    }
}

fn default_opus() -> String {
    "opus".to_string()
}

fn default_sonnet() -> String {
    "sonnet".to_string()
}

fn default_haiku() -> String {
    "haiku".to_string()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Effort {
    Low,
    Medium,
    #[default]
    High,
    Max,
}

impl Effort {
    pub fn as_str(&self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Max => "max",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct EffortConfig {
    #[serde(default)]
    pub spec: Effort,
    #[serde(default)]
    pub research: Effort,
    #[serde(default)]
    pub planning: Effort,
    #[serde(default)]
    pub review: Effort,
    #[serde(default)]
    pub execution: Effort,
    #[serde(default)]
    pub code_review: Effort,
    #[serde(default)]
    pub spec_extract: Effort,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    #[default]
    Autonomous,
    Milestones,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GitConfig {
    #[serde(default = "default_true")]
    pub auto_commit: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self { auto_commit: true }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ReviewConfig {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u8,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
        }
    }
}

fn default_max_iterations() -> u8 {
    5
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RetryConfig {
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default = "default_base_delay_secs")]
    pub base_delay_secs: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            base_delay_secs: default_base_delay_secs(),
        }
    }
}

fn default_max_retries() -> u8 {
    3
}

fn default_base_delay_secs() -> u64 {
    10
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ContextConfig {
    /// Skip spec extraction if combined spec size is below this token estimate.
    /// Set to 0 to always extract.
    #[serde(default = "default_spec_extract_threshold")]
    pub spec_extract_threshold: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            spec_extract_threshold: default_spec_extract_threshold(),
        }
    }
}

fn default_spec_extract_threshold() -> usize {
    4000
}

impl YokeConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn default_toml(project_name: &str) -> String {
        format!(
            r#"# Project identity
[project]
name = "{project_name}"

# Claude model to use for each phase. Valid values: opus, sonnet, haiku.
[models]
spec = "opus"
research = "sonnet"
planning = "opus"
review = "sonnet"
execution = "opus"
code_review = "sonnet"
spec_extract = "haiku"

# Thinking effort for each phase. Valid values: low, medium, high, max.
[effort]
spec = "high"
research = "high"
planning = "high"
review = "high"
execution = "high"
code_review = "high"
spec_extract = "high"

# Automatically commit changes at the end of each phase.
[git]
auto_commit = true

# Maximum review iterations before moving on (plan review and code review).
# If the review does not converge within this limit, behavior depends on
# interaction mode: autonomous continues anyway, milestones pauses for input.
[review]
max_iterations = 5

# Retry configuration for transient failures (rate limits, server errors).
# Uses exponential backoff: base_delay_secs * 2^attempt, with ±25% jitter.
[retry]
max_retries = 3
base_delay_secs = 10

# "autonomous" runs all phases without pausing.
# "milestones" pauses after specs, plans, and unconverged reviews for human review.
interaction = "autonomous"

# Shell commands that must pass before work is considered complete.
# These are provided to Claude as constraints, not run directly by yoke.
# Example: ["cargo fmt", "cargo check", "cargo test"]
gate_commands = []

# Skip spec extraction for small projects. If combined product + technical spec
# tokens are below this threshold, full specs are injected directly.
# Set to 0 to always extract.
[context]
spec_extract_threshold = 4000
"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_toml() {
        let toml_str = r#"
interaction = "milestones"
gate_commands = ["cargo test", "cargo clippy"]

[project]
name = "test-project"

[models]
spec = "opus"
research = "haiku"

[git]
auto_commit = false
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.project.name, "test-project");
        assert_eq!(config.models.spec, "opus");
        assert_eq!(config.models.research, "haiku");
        assert_eq!(config.models.planning, "opus");
        assert!(!config.git.auto_commit);
        assert_eq!(config.interaction, InteractionMode::Milestones);
        assert_eq!(config.gate_commands.len(), 2);
    }

    #[test]
    fn parse_invalid_toml() {
        let bad = "this is not [valid toml";
        let result = toml::from_str::<YokeConfig>(bad);
        assert!(result.is_err());
    }

    #[test]
    fn defaults_applied() {
        let minimal = r#"
[project]
name = "minimal"
"#;
        let config: YokeConfig = toml::from_str(minimal).unwrap();
        assert_eq!(config.models.spec, "opus");
        assert_eq!(config.models.research, "sonnet");
        assert_eq!(config.models.execution, "opus");
        assert_eq!(config.models.code_review, "sonnet");
        assert!(config.git.auto_commit);
        assert_eq!(config.review.max_iterations, 5);
        assert_eq!(config.retry.max_retries, 3);
        assert_eq!(config.retry.base_delay_secs, 10);
        assert_eq!(config.interaction, InteractionMode::Autonomous);
        assert!(config.gate_commands.is_empty());
        assert_eq!(config.effort.spec, Effort::High);
        assert_eq!(config.effort.research, Effort::High);
        assert_eq!(config.effort.execution, Effort::High);
        assert_eq!(config.effort.code_review, Effort::High);
    }

    #[test]
    fn default_toml_roundtrips() {
        let toml_str = YokeConfig::default_toml("roundtrip");
        let config: YokeConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.project.name, "roundtrip");
        assert_eq!(config.models.spec, "opus");
        assert!(config.git.auto_commit);
    }

    #[test]
    fn parse_effort_config() {
        let toml_str = r#"
[project]
name = "test"

[effort]
spec = "high"
research = "low"
planning = "medium"
execution = "max"
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.effort.spec, Effort::High);
        assert_eq!(config.effort.research, Effort::Low);
        assert_eq!(config.effort.planning, Effort::Medium);
        assert_eq!(config.effort.execution, Effort::Max);
        assert_eq!(config.effort.review, Effort::High);
        assert_eq!(config.effort.code_review, Effort::High);
    }

    #[test]
    fn effort_as_str() {
        assert_eq!(Effort::Low.as_str(), "low");
        assert_eq!(Effort::Medium.as_str(), "medium");
        assert_eq!(Effort::High.as_str(), "high");
        assert_eq!(Effort::Max.as_str(), "max");
    }

    #[test]
    fn load_nonexistent_file() {
        let result = YokeConfig::load(Path::new("/tmp/does_not_exist_yoke_test.toml"));
        assert!(result.is_err());
    }
}
