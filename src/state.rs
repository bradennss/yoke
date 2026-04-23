use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YokeState {
    pub spec_status: StageStatus,
    pub plan_status: StageStatus,
    #[serde(default)]
    pub spec_step: Option<SpecStep>,
    #[serde(default)]
    pub plan_step: Option<PlanStep>,
    #[serde(default)]
    pub spec_cost_usd: f64,
    #[serde(default)]
    pub plan_cost_usd: f64,
    #[serde(default)]
    pub phases: Vec<PhaseState>,
    #[serde(default)]
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    #[default]
    Pending,
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseState {
    pub number: usize,
    pub title: String,
    pub status: PhaseStatus,
    #[serde(default)]
    pub current_step: Option<PhaseStep>,
    #[serde(default)]
    pub cost_usd: f64,
    #[serde(default)]
    pub step_costs: Vec<StepCost>,
    #[serde(default)]
    pub pre_phase_commit: Option<String>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStep {
    Research,
    SpecExtract,
    Planning,
    PlanReview { iteration: u8 },
    Execution,
    CodeReview { iteration: u8 },
    Handoff,
    Commit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SpecStep {
    ProductSpecGeneration,
    ProductSpecReview { iteration: u8 },
    TechnicalSpecGeneration,
    TechnicalSpecReview { iteration: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStep {
    PlanGeneration,
    PhaseVerification,
    PlanReview { iteration: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepCost {
    pub step: String,
    pub cost_usd: f64,
}

impl YokeState {
    pub fn new() -> Self {
        Self {
            spec_status: StageStatus::Pending,
            plan_status: StageStatus::Pending,
            spec_step: None,
            plan_step: None,
            spec_cost_usd: 0.0,
            plan_cost_usd: 0.0,
            phases: Vec::new(),
            total_cost_usd: 0.0,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&content).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if path.exists() {
            let backup = path.with_extension("json.bak");
            let _ = std::fs::copy(path, &backup);
        }

        let content = serde_json::to_string_pretty(self).context("serializing state")?;

        let dir = path
            .parent()
            .context("state path has no parent directory")?;
        let temp_path = dir.join(".state.json.tmp");

        std::fs::write(&temp_path, &content)
            .with_context(|| format!("writing temp file {}", temp_path.display()))?;

        std::fs::rename(&temp_path, path)
            .with_context(|| format!("renaming {} to {}", temp_path.display(), path.display()))?;

        Ok(())
    }
}

impl Default for YokeState {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageStatus::Pending => write!(f, "pending"),
            StageStatus::InProgress => write!(f, "in progress"),
            StageStatus::Complete => write!(f, "complete"),
        }
    }
}

impl fmt::Display for PhaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseStatus::Pending => write!(f, "pending"),
            PhaseStatus::InProgress => write!(f, "in progress"),
            PhaseStatus::Completed => write!(f, "completed"),
            PhaseStatus::Failed => write!(f, "failed"),
        }
    }
}

impl fmt::Display for PhaseStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseStep::Research => write!(f, "research"),
            PhaseStep::SpecExtract => write!(f, "spec extract"),
            PhaseStep::Planning => write!(f, "planning"),
            PhaseStep::PlanReview { iteration } => write!(f, "plan review (iteration {iteration})"),
            PhaseStep::Execution => write!(f, "execution"),
            PhaseStep::CodeReview { iteration } => {
                write!(f, "code review (iteration {iteration})")
            }
            PhaseStep::Handoff => write!(f, "handoff"),
            PhaseStep::Commit => write!(f, "commit"),
        }
    }
}

impl fmt::Display for SpecStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecStep::ProductSpecGeneration => write!(f, "product spec generation"),
            SpecStep::ProductSpecReview { iteration } => {
                write!(f, "product spec review (iteration {iteration})")
            }
            SpecStep::TechnicalSpecGeneration => write!(f, "technical spec generation"),
            SpecStep::TechnicalSpecReview { iteration } => {
                write!(f, "technical spec review (iteration {iteration})")
            }
        }
    }
}

impl fmt::Display for PlanStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanStep::PlanGeneration => write!(f, "plan generation"),
            PlanStep::PhaseVerification => write!(f, "phase verification"),
            PlanStep::PlanReview { iteration } => {
                write!(f, "plan review (iteration {iteration})")
            }
        }
    }
}

pub fn spec_step_ordinal(step: &SpecStep) -> u8 {
    match step {
        SpecStep::ProductSpecGeneration => 0,
        SpecStep::ProductSpecReview { .. } => 1,
        SpecStep::TechnicalSpecGeneration => 2,
        SpecStep::TechnicalSpecReview { .. } => 3,
    }
}

pub fn plan_step_ordinal(step: &PlanStep) -> u8 {
    match step {
        PlanStep::PlanGeneration => 0,
        PlanStep::PhaseVerification => 1,
        PlanStep::PlanReview { .. } => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_roundtrip() {
        let state = YokeState {
            spec_status: StageStatus::Complete,
            plan_status: StageStatus::Complete,
            spec_step: None,
            plan_step: None,
            spec_cost_usd: 0.10,
            plan_cost_usd: 0.05,
            phases: vec![
                PhaseState {
                    number: 1,
                    title: "Setup".to_string(),
                    status: PhaseStatus::Completed,
                    current_step: None,
                    cost_usd: 0.15,
                    step_costs: vec![StepCost {
                        step: "research".to_string(),
                        cost_usd: 0.15,
                    }],
                    pre_phase_commit: Some("abc123".to_string()),
                    started_at: Some(Utc::now()),
                    completed_at: Some(Utc::now()),
                },
                PhaseState {
                    number: 2,
                    title: "Implementation".to_string(),
                    status: PhaseStatus::InProgress,
                    current_step: Some(PhaseStep::Execution),
                    cost_usd: 0.42,
                    step_costs: Vec::new(),
                    pre_phase_commit: None,
                    started_at: Some(Utc::now()),
                    completed_at: None,
                },
            ],
            total_cost_usd: 0.57,
        };

        let json = serde_json::to_string_pretty(&state).unwrap();
        let deserialized: YokeState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.spec_status, deserialized.spec_status);
        assert_eq!(state.plan_status, deserialized.plan_status);
        assert_eq!(state.phases.len(), deserialized.phases.len());
        assert_eq!(state.total_cost_usd, deserialized.total_cost_usd);

        assert_eq!(deserialized.phases[0].title, "Setup");
        assert_eq!(deserialized.phases[0].status, PhaseStatus::Completed);
        assert_eq!(deserialized.phases[1].title, "Implementation");
        assert_eq!(
            deserialized.phases[1].current_step,
            Some(PhaseStep::Execution)
        );
    }

    #[test]
    fn state_save_and_load() {
        let dir = std::env::temp_dir().join("yoke_state_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("state.json");

        let state = YokeState {
            spec_status: StageStatus::Pending,
            plan_status: StageStatus::Complete,
            spec_step: None,
            plan_step: None,
            spec_cost_usd: 0.0,
            plan_cost_usd: 0.0,
            phases: vec![PhaseState {
                number: 1,
                title: "First".to_string(),
                status: PhaseStatus::Pending,
                current_step: Some(PhaseStep::PlanReview { iteration: 2 }),
                cost_usd: 0.0,
                step_costs: Vec::new(),
                pre_phase_commit: None,
                started_at: None,
                completed_at: None,
            }],
            total_cost_usd: 0.0,
        };

        state.save(&path).unwrap();
        let loaded = YokeState::load(&path).unwrap();

        assert_eq!(state.spec_status, loaded.spec_status);
        assert_eq!(state.plan_status, loaded.plan_status);
        assert_eq!(state.phases.len(), loaded.phases.len());
        assert_eq!(
            loaded.phases[0].current_step,
            Some(PhaseStep::PlanReview { iteration: 2 })
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn phase_status_transitions() {
        let mut phase = PhaseState {
            number: 1,
            title: "Test".to_string(),
            status: PhaseStatus::Pending,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: None,
        };

        assert_eq!(phase.status, PhaseStatus::Pending);

        phase.status = PhaseStatus::InProgress;
        phase.current_step = Some(PhaseStep::Research);
        phase.started_at = Some(Utc::now());
        assert_eq!(phase.status, PhaseStatus::InProgress);
        assert_eq!(phase.current_step, Some(PhaseStep::Research));

        phase.current_step = Some(PhaseStep::CodeReview { iteration: 1 });
        assert_eq!(
            phase.current_step,
            Some(PhaseStep::CodeReview { iteration: 1 })
        );

        phase.status = PhaseStatus::Completed;
        phase.current_step = None;
        phase.completed_at = Some(Utc::now());
        assert_eq!(phase.status, PhaseStatus::Completed);
        assert!(phase.started_at.is_some());
        assert!(phase.completed_at.is_some());
    }

    #[test]
    fn state_save_creates_backup() {
        let dir = std::env::temp_dir().join("yoke_state_backup_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("state.json");
        let backup_path = dir.join("state.json.bak");

        let state_v1 = YokeState {
            total_cost_usd: 1.0,
            ..YokeState::new()
        };
        state_v1.save(&path).unwrap();
        assert!(!backup_path.exists());

        let state_v2 = YokeState {
            spec_status: StageStatus::Complete,
            total_cost_usd: 2.0,
            ..YokeState::new()
        };
        state_v2.save(&path).unwrap();
        assert!(backup_path.exists());

        let backup = YokeState::load(&backup_path).unwrap();
        assert_eq!(backup.total_cost_usd, 1.0);
        assert_eq!(backup.spec_status, StageStatus::Pending);

        let current = YokeState::load(&path).unwrap();
        assert_eq!(current.total_cost_usd, 2.0);
        assert_eq!(current.spec_status, StageStatus::Complete);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn new_state_is_empty() {
        let state = YokeState::new();
        assert_eq!(state.spec_status, StageStatus::Pending);
        assert_eq!(state.plan_status, StageStatus::Pending);
        assert!(state.phases.is_empty());
        assert_eq!(state.total_cost_usd, 0.0);
    }

    #[test]
    fn stage_status_display() {
        assert_eq!(StageStatus::Pending.to_string(), "pending");
        assert_eq!(StageStatus::InProgress.to_string(), "in progress");
        assert_eq!(StageStatus::Complete.to_string(), "complete");
    }

    #[test]
    fn phase_status_display() {
        assert_eq!(PhaseStatus::Pending.to_string(), "pending");
        assert_eq!(PhaseStatus::InProgress.to_string(), "in progress");
        assert_eq!(PhaseStatus::Completed.to_string(), "completed");
        assert_eq!(PhaseStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn phase_step_display() {
        assert_eq!(PhaseStep::Research.to_string(), "research");
        assert_eq!(PhaseStep::SpecExtract.to_string(), "spec extract");
        assert_eq!(
            PhaseStep::PlanReview { iteration: 3 }.to_string(),
            "plan review (iteration 3)"
        );
        assert_eq!(
            PhaseStep::CodeReview { iteration: 1 }.to_string(),
            "code review (iteration 1)"
        );
    }

    #[test]
    fn spec_step_display() {
        assert_eq!(
            SpecStep::ProductSpecGeneration.to_string(),
            "product spec generation"
        );
        assert_eq!(
            SpecStep::ProductSpecReview { iteration: 2 }.to_string(),
            "product spec review (iteration 2)"
        );
        assert_eq!(
            SpecStep::TechnicalSpecGeneration.to_string(),
            "technical spec generation"
        );
        assert_eq!(
            SpecStep::TechnicalSpecReview { iteration: 1 }.to_string(),
            "technical spec review (iteration 1)"
        );
    }

    #[test]
    fn plan_step_display() {
        assert_eq!(PlanStep::PlanGeneration.to_string(), "plan generation");
        assert_eq!(
            PlanStep::PhaseVerification.to_string(),
            "phase verification"
        );
        assert_eq!(
            PlanStep::PlanReview { iteration: 3 }.to_string(),
            "plan review (iteration 3)"
        );
    }

    #[test]
    fn spec_step_ordinals() {
        assert_eq!(spec_step_ordinal(&SpecStep::ProductSpecGeneration), 0);
        assert_eq!(
            spec_step_ordinal(&SpecStep::ProductSpecReview { iteration: 5 }),
            1
        );
        assert_eq!(spec_step_ordinal(&SpecStep::TechnicalSpecGeneration), 2);
        assert_eq!(
            spec_step_ordinal(&SpecStep::TechnicalSpecReview { iteration: 1 }),
            3
        );
    }

    #[test]
    fn plan_step_ordinals() {
        assert_eq!(plan_step_ordinal(&PlanStep::PlanGeneration), 0);
        assert_eq!(plan_step_ordinal(&PlanStep::PhaseVerification), 1);
        assert_eq!(plan_step_ordinal(&PlanStep::PlanReview { iteration: 2 }), 2);
    }

    #[test]
    fn step_cost_serde_roundtrip() {
        let costs = vec![
            StepCost {
                step: "research".to_string(),
                cost_usd: 0.12,
            },
            StepCost {
                step: "execution".to_string(),
                cost_usd: 0.38,
            },
        ];
        let json = serde_json::to_string(&costs).unwrap();
        let deserialized: Vec<StepCost> = serde_json::from_str(&json).unwrap();
        assert_eq!(costs, deserialized);
    }

    #[test]
    fn backward_compatible_deserialization() {
        let old_json = r#"{
            "spec_status": "pending",
            "plan_status": "complete",
            "phases": [{
                "number": 1,
                "title": "Setup",
                "status": "pending",
                "cost_usd": 0.0
            }],
            "total_cost_usd": 0.0
        }"#;
        let state: YokeState = serde_json::from_str(old_json).unwrap();
        assert_eq!(state.spec_step, None);
        assert_eq!(state.plan_step, None);
        assert_eq!(state.spec_cost_usd, 0.0);
        assert_eq!(state.plan_cost_usd, 0.0);
        assert!(state.phases[0].step_costs.is_empty());
        assert_eq!(state.phases[0].pre_phase_commit, None);
    }

    #[test]
    fn in_progress_status_serde() {
        let state = YokeState {
            spec_status: StageStatus::InProgress,
            spec_step: Some(SpecStep::ProductSpecReview { iteration: 2 }),
            ..YokeState::new()
        };
        let json = serde_json::to_string_pretty(&state).unwrap();
        let deserialized: YokeState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.spec_status, StageStatus::InProgress);
        assert_eq!(
            deserialized.spec_step,
            Some(SpecStep::ProductSpecReview { iteration: 2 })
        );
    }
}
