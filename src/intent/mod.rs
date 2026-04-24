pub mod migrate;
pub mod store;

use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::{PhaseState, PlanStep, SpecStep, StageStatus};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Build,
    Feature,
    Fix,
    Refactor,
    Maintenance,
}

impl fmt::Display for Classification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Classification::Build => write!(f, "build"),
            Classification::Feature => write!(f, "feature"),
            Classification::Fix => write!(f, "fix"),
            Classification::Refactor => write!(f, "refactor"),
            Classification::Maintenance => write!(f, "maintenance"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Depth {
    Full,
    Light,
    Minimal,
}

impl fmt::Display for Depth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Depth::Full => write!(f, "full"),
            Depth::Light => write!(f, "light"),
            Depth::Minimal => write!(f, "minimal"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

impl fmt::Display for IntentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentStatus::Pending => write!(f, "pending"),
            IntentStatus::InProgress => write!(f, "in progress"),
            IntentStatus::Completed => write!(f, "completed"),
            IntentStatus::Failed => write!(f, "failed"),
            IntentStatus::Blocked => write!(f, "blocked"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentState {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,

    pub classification: Classification,
    pub depth: Depth,

    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub blocked_by: Vec<String>,

    #[serde(default)]
    pub status: IntentStatus,

    #[serde(default)]
    pub spec_status: StageStatus,
    #[serde(default)]
    pub spec_step: Option<SpecStep>,
    #[serde(default)]
    pub plan_status: StageStatus,
    #[serde(default)]
    pub plan_step: Option<PlanStep>,
    #[serde(default)]
    pub phases: Vec<PhaseState>,

    #[serde(default)]
    pub spec_cost_usd: f64,
    #[serde(default)]
    pub plan_cost_usd: f64,
    #[serde(default)]
    pub total_cost_usd: f64,

    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
}

impl IntentState {
    pub fn new(
        number: u32,
        title: String,
        description: String,
        classification: Classification,
        depth: Depth,
    ) -> Self {
        let slug = slugify(&title);
        Self {
            id: format!("i-{number:03}"),
            slug,
            title,
            description,
            classification,
            depth,
            branch: None,
            worktree_path: None,
            parent: None,
            blocked_by: Vec::new(),
            status: IntentStatus::Pending,
            spec_status: StageStatus::default(),
            spec_step: None,
            plan_status: StageStatus::default(),
            plan_step: None,
            phases: Vec::new(),
            spec_cost_usd: 0.0,
            plan_cost_usd: 0.0,
            total_cost_usd: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn dir_name(&self) -> String {
        format!("{}-{}", self.id, self.slug)
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

        let content = serde_json::to_string_pretty(self).context("serializing intent state")?;

        let dir = path
            .parent()
            .context("intent state path has no parent directory")?;
        let temp_path = dir.join(".intent.json.tmp");

        std::fs::write(&temp_path, &content)
            .with_context(|| format!("writing temp file {}", temp_path.display()))?;

        std::fs::rename(&temp_path, path)
            .with_context(|| format!("renaming {} to {}", temp_path.display(), path.display()))?;

        Ok(())
    }
}

pub fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut prev_hyphen = true; // prevent leading hyphen

    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_display() {
        assert_eq!(Classification::Build.to_string(), "build");
        assert_eq!(Classification::Feature.to_string(), "feature");
        assert_eq!(Classification::Fix.to_string(), "fix");
        assert_eq!(Classification::Refactor.to_string(), "refactor");
        assert_eq!(Classification::Maintenance.to_string(), "maintenance");
    }

    #[test]
    fn classification_serde_roundtrip() {
        for variant in [
            Classification::Build,
            Classification::Feature,
            Classification::Fix,
            Classification::Refactor,
            Classification::Maintenance,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: Classification = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn depth_display() {
        assert_eq!(Depth::Full.to_string(), "full");
        assert_eq!(Depth::Light.to_string(), "light");
        assert_eq!(Depth::Minimal.to_string(), "minimal");
    }

    #[test]
    fn depth_serde_roundtrip() {
        for variant in [Depth::Full, Depth::Light, Depth::Minimal] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: Depth = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn intent_status_display() {
        assert_eq!(IntentStatus::Pending.to_string(), "pending");
        assert_eq!(IntentStatus::InProgress.to_string(), "in progress");
        assert_eq!(IntentStatus::Completed.to_string(), "completed");
        assert_eq!(IntentStatus::Failed.to_string(), "failed");
        assert_eq!(IntentStatus::Blocked.to_string(), "blocked");
    }

    #[test]
    fn intent_status_default() {
        assert_eq!(IntentStatus::default(), IntentStatus::Pending);
    }

    #[test]
    fn intent_status_serde_roundtrip() {
        for variant in [
            IntentStatus::Pending,
            IntentStatus::InProgress,
            IntentStatus::Completed,
            IntentStatus::Failed,
            IntentStatus::Blocked,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: IntentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn new_intent_state() {
        let intent = IntentState::new(
            1,
            "Initial build".to_string(),
            "Build the whole thing".to_string(),
            Classification::Build,
            Depth::Full,
        );

        assert_eq!(intent.id, "i-001");
        assert_eq!(intent.slug, "initial-build");
        assert_eq!(intent.title, "Initial build");
        assert_eq!(intent.classification, Classification::Build);
        assert_eq!(intent.depth, Depth::Full);
        assert_eq!(intent.status, IntentStatus::Pending);
        assert_eq!(intent.spec_status, StageStatus::Pending);
        assert_eq!(intent.plan_status, StageStatus::Pending);
        assert!(intent.phases.is_empty());
        assert_eq!(intent.total_cost_usd, 0.0);
        assert!(intent.started_at.is_none());
        assert!(intent.completed_at.is_none());
    }

    #[test]
    fn dir_name_format() {
        let intent = IntentState::new(
            3,
            "Auth feature".to_string(),
            "Add authentication".to_string(),
            Classification::Feature,
            Depth::Light,
        );
        assert_eq!(intent.dir_name(), "i-003-auth-feature");
    }

    #[test]
    fn dir_name_high_number() {
        let intent = IntentState::new(
            42,
            "Cleanup".to_string(),
            "Clean up code".to_string(),
            Classification::Maintenance,
            Depth::Minimal,
        );
        assert_eq!(intent.dir_name(), "i-042-cleanup");
    }

    #[test]
    fn intent_state_serde_roundtrip() {
        let intent = IntentState::new(
            5,
            "Fix login bug".to_string(),
            "Users can't log in when session expires".to_string(),
            Classification::Fix,
            Depth::Minimal,
        );

        let json = serde_json::to_string_pretty(&intent).unwrap();
        let parsed: IntentState = serde_json::from_str(&json).unwrap();

        assert_eq!(intent.id, parsed.id);
        assert_eq!(intent.slug, parsed.slug);
        assert_eq!(intent.title, parsed.title);
        assert_eq!(intent.description, parsed.description);
        assert_eq!(intent.classification, parsed.classification);
        assert_eq!(intent.depth, parsed.depth);
        assert_eq!(intent.status, parsed.status);
    }

    #[test]
    fn intent_state_save_and_load() {
        let dir = std::env::temp_dir().join("yoke_intent_save_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("intent.json");

        let intent = IntentState::new(
            1,
            "Test intent".to_string(),
            "Testing save and load".to_string(),
            Classification::Feature,
            Depth::Light,
        );
        intent.save(&path).unwrap();

        let loaded = IntentState::load(&path).unwrap();
        assert_eq!(intent.id, loaded.id);
        assert_eq!(intent.slug, loaded.slug);
        assert_eq!(intent.classification, loaded.classification);
        assert_eq!(intent.depth, loaded.depth);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn intent_state_save_creates_backup() {
        let dir = std::env::temp_dir().join("yoke_intent_backup_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("intent.json");
        let backup_path = dir.join("intent.json.bak");

        let v1 = IntentState::new(
            1,
            "V1".to_string(),
            "First version".to_string(),
            Classification::Build,
            Depth::Full,
        );
        v1.save(&path).unwrap();
        assert!(!backup_path.exists());

        let mut v2 = IntentState::new(
            1,
            "V2".to_string(),
            "Second version".to_string(),
            Classification::Build,
            Depth::Full,
        );
        v2.total_cost_usd = 1.50;
        v2.save(&path).unwrap();
        assert!(backup_path.exists());

        let backup = IntentState::load(&backup_path).unwrap();
        assert_eq!(backup.title, "V1");

        let current = IntentState::load(&path).unwrap();
        assert_eq!(current.title, "V2");
        assert_eq!(current.total_cost_usd, 1.50);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn intent_state_backward_compatible() {
        let json = r#"{
            "id": "i-001",
            "slug": "test",
            "title": "Test",
            "description": "A test",
            "classification": "build",
            "depth": "full",
            "created_at": "2025-01-01T00:00:00Z"
        }"#;
        let intent: IntentState = serde_json::from_str(json).unwrap();
        assert_eq!(intent.status, IntentStatus::Pending);
        assert_eq!(intent.spec_status, StageStatus::Pending);
        assert!(intent.phases.is_empty());
        assert_eq!(intent.total_cost_usd, 0.0);
        assert!(intent.branch.is_none());
        assert!(intent.blocked_by.is_empty());
    }

    #[test]
    fn slugify_simple() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_special_characters() {
        assert_eq!(slugify("Fix: login bug!"), "fix-login-bug");
    }

    #[test]
    fn slugify_multiple_spaces() {
        assert_eq!(slugify("too   many   spaces"), "too-many-spaces");
    }

    #[test]
    fn slugify_leading_trailing() {
        assert_eq!(slugify("  hello  "), "hello");
    }

    #[test]
    fn slugify_mixed_case() {
        assert_eq!(slugify("MyFeature"), "myfeature");
    }

    #[test]
    fn slugify_numbers() {
        assert_eq!(slugify("Phase 3 setup"), "phase-3-setup");
    }

    #[test]
    fn slugify_all_special() {
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn slugify_already_slug() {
        assert_eq!(slugify("already-a-slug"), "already-a-slug");
    }
}
