use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct YokeConfig {
    pub project: ProjectConfig,

    // -- Per-workflow config (new) ------------------------------------------
    #[serde(default)]
    pub discover: WorkflowConfig,
    #[serde(default)]
    pub spec: WorkflowConfig,
    #[serde(default)]
    pub plan: WorkflowConfig,
    #[serde(default)]
    pub phase: PhaseConfig,
    #[serde(default)]
    pub classify: ClassifyConfig,

    // -- Cross-cutting settings ---------------------------------------------
    #[serde(default)]
    pub gate_commands: Vec<String>,
    #[serde(default)]
    pub interaction: InteractionMode,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub worktrees: WorktreeConfig,
    #[serde(default)]
    pub knowledge: KnowledgeConfig,
    #[serde(default)]
    pub retry: RetryConfig,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ProjectConfig {
    pub name: String,
}

// ---------------------------------------------------------------------------
// Per-workflow config types
// ---------------------------------------------------------------------------

/// Shared config shape for top-level workflows (discover, spec, plan).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowConfig {
    #[serde(default = "default_opus")]
    pub model: String,
    #[serde(default)]
    pub effort: Effort,
    #[serde(default = "default_sonnet")]
    pub review_model: String,
    #[serde(default)]
    pub review_effort: Effort,
    #[serde(default = "default_max_review_iterations")]
    pub max_review_iterations: u8,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            model: default_opus(),
            effort: Effort::High,
            review_model: default_sonnet(),
            review_effort: Effort::High,
            max_review_iterations: default_max_review_iterations(),
        }
    }
}

/// Config for a single phase sub-step (research, planning, execution, handoff).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepConfig {
    #[serde(default = "default_sonnet")]
    pub model: String,
    #[serde(default)]
    pub effort: Effort,
}

impl StepConfig {
    fn with_defaults(model: fn() -> String, effort: Effort) -> Self {
        Self {
            model: model(),
            effort,
        }
    }
}

/// Config for review sub-steps within a phase (plan_review, code_review).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewStepConfig {
    #[serde(default = "default_sonnet")]
    pub model: String,
    #[serde(default)]
    pub effort: Effort,
    #[serde(default = "default_max_review_iterations")]
    pub max_iterations: u8,
}

impl Default for ReviewStepConfig {
    fn default() -> Self {
        Self {
            model: default_sonnet(),
            effort: Effort::High,
            max_iterations: default_max_review_iterations(),
        }
    }
}

/// Config for the spec extraction sub-step within a phase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecExtractConfig {
    #[serde(default = "default_haiku")]
    pub model: String,
    #[serde(default)]
    pub effort: Effort,
    #[serde(default = "default_spec_extract_threshold")]
    pub threshold: usize,
}

impl Default for SpecExtractConfig {
    fn default() -> Self {
        Self {
            model: default_haiku(),
            effort: Effort::High,
            threshold: default_spec_extract_threshold(),
        }
    }
}

/// Per-step config for the phase workflow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseConfig {
    #[serde(default = "default_research_step")]
    pub research: StepConfig,
    #[serde(default)]
    pub spec_extract: SpecExtractConfig,
    #[serde(default = "default_planning_step")]
    pub planning: StepConfig,
    #[serde(default)]
    pub plan_review: ReviewStepConfig,
    #[serde(default = "default_execution_step")]
    pub execution: StepConfig,
    #[serde(default)]
    pub code_review: ReviewStepConfig,
    #[serde(default = "default_handoff_step")]
    pub handoff: StepConfig,
}

impl Default for PhaseConfig {
    fn default() -> Self {
        Self {
            research: default_research_step(),
            spec_extract: SpecExtractConfig::default(),
            planning: default_planning_step(),
            plan_review: ReviewStepConfig::default(),
            execution: default_execution_step(),
            code_review: ReviewStepConfig::default(),
            handoff: default_handoff_step(),
        }
    }
}

fn default_research_step() -> StepConfig {
    StepConfig::with_defaults(default_sonnet, Effort::High)
}

fn default_planning_step() -> StepConfig {
    StepConfig::with_defaults(default_opus, Effort::High)
}

fn default_execution_step() -> StepConfig {
    StepConfig::with_defaults(default_opus, Effort::High)
}

fn default_handoff_step() -> StepConfig {
    StepConfig::with_defaults(default_sonnet, Effort::High)
}

/// Config for intent auto-classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassifyConfig {
    #[serde(default = "default_haiku")]
    pub model: String,
}

impl Default for ClassifyConfig {
    fn default() -> Self {
        Self {
            model: default_haiku(),
        }
    }
}

/// Git worktree directory configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorktreeConfig {
    #[serde(default = "default_worktree_directory")]
    pub directory: String,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            directory: default_worktree_directory(),
        }
    }
}

fn default_worktree_directory() -> String {
    "../{project}-worktrees".to_string()
}

/// Knowledge base configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeConfig {
    #[serde(default = "default_compaction_interval")]
    pub compaction_interval: usize,
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            compaction_interval: default_compaction_interval(),
        }
    }
}

fn default_compaction_interval() -> usize {
    10
}

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

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

    pub fn reduced(self) -> Self {
        match self {
            Effort::Max => Effort::High,
            Effort::High => Effort::Medium,
            Effort::Medium => Effort::Low,
            Effort::Low => Effort::Low,
        }
    }
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
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            auto_commit: true,
            branch_prefix: default_branch_prefix(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_branch_prefix() -> String {
    "yoke".to_string()
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

// ---------------------------------------------------------------------------
// Default value helpers
// ---------------------------------------------------------------------------

fn default_opus() -> String {
    "opus".to_string()
}

fn default_sonnet() -> String {
    "sonnet".to_string()
}

fn default_haiku() -> String {
    "haiku".to_string()
}

fn default_max_review_iterations() -> u8 {
    5
}

fn default_spec_extract_threshold() -> usize {
    4000
}

// ---------------------------------------------------------------------------
// Loading and serialization
// ---------------------------------------------------------------------------

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

# Discovery workflow (yoke discover): explores existing codebase to generate specs.
[discover]
model = "opus"
effort = "high"
review_model = "sonnet"
review_effort = "high"
max_review_iterations = 5

# Spec workflow (yoke new --class build): generates product and technical specs.
[spec]
model = "opus"
effort = "high"
review_model = "sonnet"
review_effort = "high"
max_review_iterations = 5

# Plan workflow: generates implementation plan and phases.
[plan]
model = "opus"
effort = "high"
review_model = "sonnet"
review_effort = "high"
max_review_iterations = 5

# Phase sub-step configuration.
[phase.research]
model = "sonnet"
effort = "high"

[phase.spec_extract]
model = "haiku"
effort = "high"
threshold = 4000

[phase.planning]
model = "opus"
effort = "high"

[phase.plan_review]
model = "sonnet"
effort = "high"
max_iterations = 5

[phase.execution]
model = "opus"
effort = "high"

[phase.code_review]
model = "sonnet"
effort = "high"
max_iterations = 5

[phase.handoff]
model = "sonnet"
effort = "high"

# Intent auto-classification model.
[classify]
model = "haiku"

# Automatically commit changes at the end of each phase.
[git]
auto_commit = true
branch_prefix = "yoke"

# Git worktree directory for parallel intents.
[worktrees]
directory = "../{{project}}-worktrees"

# Knowledge base compaction interval (number of intents between compactions).
[knowledge]
compaction_interval = 10

# Retry configuration for transient failures (rate limits, server errors).
# Uses exponential backoff: base_delay_secs * 2^attempt, with +/- 25% jitter.
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

[spec]
model = "opus"

[phase.research]
model = "haiku"

[git]
auto_commit = false
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.project.name, "test-project");
        assert_eq!(config.spec.model, "opus");
        assert_eq!(config.phase.research.model, "haiku");
        assert_eq!(config.plan.model, "opus");
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
        assert_eq!(config.spec.model, "opus");
        assert_eq!(config.phase.research.model, "sonnet");
        assert_eq!(config.phase.execution.model, "opus");
        assert_eq!(config.phase.code_review.model, "sonnet");
        assert!(config.git.auto_commit);
        assert_eq!(config.spec.max_review_iterations, 5);
        assert_eq!(config.retry.max_retries, 3);
        assert_eq!(config.retry.base_delay_secs, 10);
        assert_eq!(config.interaction, InteractionMode::Autonomous);
        assert!(config.gate_commands.is_empty());
        assert_eq!(config.spec.effort, Effort::High);
        assert_eq!(config.phase.research.effort, Effort::High);
        assert_eq!(config.phase.execution.effort, Effort::High);
        assert_eq!(config.phase.code_review.effort, Effort::High);
    }

    #[test]
    fn default_toml_roundtrips() {
        let toml_str = YokeConfig::default_toml("roundtrip");
        let config: YokeConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.project.name, "roundtrip");
        assert_eq!(config.spec.model, "opus");
        assert_eq!(config.phase.execution.model, "opus");
        assert!(config.git.auto_commit);
    }

    #[test]
    fn parse_effort_config() {
        let toml_str = r#"
[project]
name = "test"

[spec]
effort = "high"

[phase.research]
effort = "low"

[phase.planning]
effort = "medium"

[phase.execution]
effort = "max"
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.spec.effort, Effort::High);
        assert_eq!(config.phase.research.effort, Effort::Low);
        assert_eq!(config.phase.planning.effort, Effort::Medium);
        assert_eq!(config.phase.execution.effort, Effort::Max);
        assert_eq!(config.phase.plan_review.effort, Effort::High);
        assert_eq!(config.phase.code_review.effort, Effort::High);
    }

    #[test]
    fn effort_as_str() {
        assert_eq!(Effort::Low.as_str(), "low");
        assert_eq!(Effort::Medium.as_str(), "medium");
        assert_eq!(Effort::High.as_str(), "high");
        assert_eq!(Effort::Max.as_str(), "max");
    }

    #[test]
    fn effort_reduced() {
        assert_eq!(Effort::Max.reduced(), Effort::High);
        assert_eq!(Effort::High.reduced(), Effort::Medium);
        assert_eq!(Effort::Medium.reduced(), Effort::Low);
        assert_eq!(Effort::Low.reduced(), Effort::Low);
    }

    #[test]
    fn load_nonexistent_file() {
        let result = YokeConfig::load(Path::new("/tmp/does_not_exist_yoke_test.toml"));
        assert!(result.is_err());
    }

    // -- New per-workflow config tests --------------------------------------

    #[test]
    fn parse_new_workflow_config() {
        let toml_str = r#"
[project]
name = "new-style"

[discover]
model = "opus"
effort = "max"
review_model = "sonnet"
review_effort = "medium"
max_review_iterations = 3

[spec]
model = "opus"
effort = "high"

[plan]
model = "sonnet"
review_model = "haiku"
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.discover.model, "opus");
        assert_eq!(config.discover.effort, Effort::Max);
        assert_eq!(config.discover.review_model, "sonnet");
        assert_eq!(config.discover.review_effort, Effort::Medium);
        assert_eq!(config.discover.max_review_iterations, 3);

        assert_eq!(config.spec.model, "opus");
        assert_eq!(config.spec.effort, Effort::High);
        assert_eq!(config.spec.review_model, "sonnet");
        assert_eq!(config.spec.max_review_iterations, 5);

        assert_eq!(config.plan.model, "sonnet");
        assert_eq!(config.plan.review_model, "haiku");
    }

    #[test]
    fn parse_phase_config() {
        let toml_str = r#"
[project]
name = "phase-test"

[phase.research]
model = "haiku"
effort = "low"

[phase.execution]
model = "opus"
effort = "max"

[phase.code_review]
model = "sonnet"
effort = "high"
max_iterations = 3

[phase.spec_extract]
model = "haiku"
threshold = 8000
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.phase.research.model, "haiku");
        assert_eq!(config.phase.research.effort, Effort::Low);
        assert_eq!(config.phase.execution.model, "opus");
        assert_eq!(config.phase.execution.effort, Effort::Max);
        assert_eq!(config.phase.code_review.model, "sonnet");
        assert_eq!(config.phase.code_review.max_iterations, 3);
        assert_eq!(config.phase.spec_extract.threshold, 8000);

        // Defaults for unspecified steps.
        assert_eq!(config.phase.planning.model, "opus");
        assert_eq!(config.phase.plan_review.max_iterations, 5);
        assert_eq!(config.phase.handoff.model, "sonnet");
    }

    #[test]
    fn parse_classify_config() {
        let toml_str = r#"
[project]
name = "classify-test"

[classify]
model = "sonnet"
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.classify.model, "sonnet");
    }

    #[test]
    fn classify_defaults_to_haiku() {
        let minimal = r#"
[project]
name = "minimal"
"#;
        let config: YokeConfig = toml::from_str(minimal).unwrap();
        assert_eq!(config.classify.model, "haiku");
    }

    #[test]
    fn parse_worktree_config() {
        let toml_str = r#"
[project]
name = "wt-test"

[worktrees]
directory = "/tmp/worktrees"
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.worktrees.directory, "/tmp/worktrees");
    }

    #[test]
    fn worktree_defaults() {
        let minimal = r#"
[project]
name = "minimal"
"#;
        let config: YokeConfig = toml::from_str(minimal).unwrap();
        assert_eq!(config.worktrees.directory, "../{project}-worktrees");
    }

    #[test]
    fn parse_knowledge_config() {
        let toml_str = r#"
[project]
name = "kb-test"

[knowledge]
compaction_interval = 20
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.knowledge.compaction_interval, 20);
    }

    #[test]
    fn knowledge_defaults() {
        let minimal = r#"
[project]
name = "minimal"
"#;
        let config: YokeConfig = toml::from_str(minimal).unwrap();
        assert_eq!(config.knowledge.compaction_interval, 10);
    }

    #[test]
    fn git_branch_prefix() {
        let toml_str = r#"
[project]
name = "git-test"

[git]
auto_commit = true
branch_prefix = "feat"
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.git.branch_prefix, "feat");
    }

    #[test]
    fn git_branch_prefix_defaults() {
        let minimal = r#"
[project]
name = "minimal"
"#;
        let config: YokeConfig = toml::from_str(minimal).unwrap();
        assert_eq!(config.git.branch_prefix, "yoke");
    }

    #[test]
    fn per_workflow_config_sections() {
        let toml_str = r#"
[project]
name = "mixed"

[spec]
model = "opus"
effort = "max"

[phase.execution]
model = "opus"
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.spec.model, "opus");
        assert_eq!(config.spec.effort, Effort::Max);
        assert_eq!(config.phase.execution.model, "opus");
    }

    #[test]
    fn minimal_config_new_defaults() {
        let minimal = r#"
[project]
name = "minimal"
"#;
        let config: YokeConfig = toml::from_str(minimal).unwrap();

        // Per-workflow defaults.
        assert_eq!(config.discover.model, "opus");
        assert_eq!(config.discover.review_model, "sonnet");
        assert_eq!(config.discover.max_review_iterations, 5);
        assert_eq!(config.spec.model, "opus");
        assert_eq!(config.plan.model, "opus");

        // Phase defaults.
        assert_eq!(config.phase.research.model, "sonnet");
        assert_eq!(config.phase.spec_extract.model, "haiku");
        assert_eq!(config.phase.spec_extract.threshold, 4000);
        assert_eq!(config.phase.planning.model, "opus");
        assert_eq!(config.phase.plan_review.model, "sonnet");
        assert_eq!(config.phase.plan_review.max_iterations, 5);
        assert_eq!(config.phase.execution.model, "opus");
        assert_eq!(config.phase.code_review.model, "sonnet");
        assert_eq!(config.phase.code_review.max_iterations, 5);
        assert_eq!(config.phase.handoff.model, "sonnet");

        // Classify default.
        assert_eq!(config.classify.model, "haiku");
    }
}
