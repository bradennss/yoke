use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::YokeConfig;
use crate::state::{PhaseState, PhaseStatus, PlanStep, SpecStep, StageStatus};

use super::{Classification, Depth, IntentState, IntentStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct YokeState {
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

#[allow(dead_code)]
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

pub fn needs_migration(project_dir: &Path) -> bool {
    let state_path = project_dir.join(".yoke/state.json");
    let intents_dir = project_dir.join(".yoke/intents");
    state_path.exists() && !intents_dir.exists()
}

pub fn migrate(project_dir: &Path, config: &YokeConfig) -> Result<IntentState> {
    let state_path = project_dir.join(".yoke/state.json");
    let old_state = YokeState::load(&state_path).context("loading old state for migration")?;

    let mut intent = IntentState::new(
        1,
        "initial build".to_string(),
        format!("{} initial build", config.project.name),
        Classification::Build,
        Depth::Full,
    );

    intent.spec_status = old_state.spec_status;
    intent.spec_step = old_state.spec_step;
    intent.plan_status = old_state.plan_status;
    intent.plan_step = old_state.plan_step;
    intent.phases = old_state.phases;
    intent.spec_cost_usd = old_state.spec_cost_usd;
    intent.plan_cost_usd = old_state.plan_cost_usd;
    intent.total_cost_usd = old_state.total_cost_usd;

    intent.status = derive_status(&intent.phases);

    let specs_dir = project_dir.join(".yoke/specs");
    let intent_dir = project_dir.join(".yoke/intents").join(intent.dir_name());

    std::fs::create_dir_all(&specs_dir)
        .with_context(|| format!("creating {}", specs_dir.display()))?;
    for subdir in ["phases", "research", "plans", "handoffs"] {
        let path = intent_dir.join(subdir);
        std::fs::create_dir_all(&path).with_context(|| format!("creating {}", path.display()))?;
    }

    let docs_dir = project_dir.join("docs");

    move_file(
        &docs_dir.join("product-spec.md"),
        &specs_dir.join("product.md"),
    )?;
    move_file(
        &docs_dir.join("technical-spec.md"),
        &specs_dir.join("technical.md"),
    )?;
    move_file(&docs_dir.join("plan.md"), &intent_dir.join("plan.md"))?;
    move_dir_contents(&docs_dir.join("phases"), &intent_dir.join("phases"))?;
    move_dir_contents(&docs_dir.join("research"), &intent_dir.join("research"))?;
    move_dir_contents(&docs_dir.join("plans"), &intent_dir.join("plans"))?;
    move_dir_contents(&docs_dir.join("handoffs"), &intent_dir.join("handoffs"))?;

    delete_dir_if_exists(&docs_dir.join("extracts"))?;
    delete_dir_if_exists(&docs_dir.join("summaries"))?;

    if docs_dir.exists() && is_dir_empty(&docs_dir)? {
        std::fs::remove_dir(&docs_dir)
            .with_context(|| format!("removing empty {}", docs_dir.display()))?;
    }

    let intent_json = intent_dir.join("intent.json");
    intent.save(&intent_json)?;

    let migrated_path = project_dir.join(".yoke/state.json.migrated");
    std::fs::copy(&state_path, &migrated_path).with_context(|| {
        format!(
            "copying {} to {}",
            state_path.display(),
            migrated_path.display()
        )
    })?;
    std::fs::remove_file(&state_path)
        .with_context(|| format!("removing {}", state_path.display()))?;

    eprintln!(
        "Migrated legacy state to intent {} ({})",
        intent.id,
        intent.dir_name()
    );

    Ok(intent)
}

fn derive_status(phases: &[PhaseState]) -> IntentStatus {
    if phases.is_empty() {
        return IntentStatus::Pending;
    }

    let all_completed = phases.iter().all(|p| p.status == PhaseStatus::Completed);
    if all_completed {
        return IntentStatus::Completed;
    }

    let any_in_progress = phases.iter().any(|p| p.status == PhaseStatus::InProgress);
    if any_in_progress {
        return IntentStatus::InProgress;
    }

    IntentStatus::Pending
}

fn move_file(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::copy(src, dst)
        .with_context(|| format!("copying {} to {}", src.display(), dst.display()))?;
    std::fs::remove_file(src).with_context(|| format!("removing {}", src.display()))?;
    Ok(())
}

fn move_dir_contents(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    if !src_dir.exists() {
        return Ok(());
    }
    let entries =
        std::fs::read_dir(src_dir).with_context(|| format!("reading {}", src_dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst_dir.join(&file_name);
        std::fs::copy(&src_path, &dst_path)
            .with_context(|| format!("copying {} to {}", src_path.display(), dst_path.display()))?;
        std::fs::remove_file(&src_path)
            .with_context(|| format!("removing {}", src_path.display()))?;
    }
    if is_dir_empty(src_dir)? {
        std::fs::remove_dir(src_dir)
            .with_context(|| format!("removing empty {}", src_dir.display()))?;
    }
    Ok(())
}

fn delete_dir_if_exists(dir: &Path) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    Ok(())
}

fn is_dir_empty(dir: &Path) -> Result<bool> {
    let mut entries =
        std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    Ok(entries.next().is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yoke_migrate_test_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn minimal_config(name: &str) -> YokeConfig {
        let toml_str = format!(
            r#"
[project]
name = "{name}"
"#
        );
        toml::from_str(&toml_str).unwrap()
    }

    fn write_old_state(dir: &Path, state: &YokeState) {
        let yoke_dir = dir.join(".yoke");
        std::fs::create_dir_all(&yoke_dir).unwrap();
        state.save(&yoke_dir.join("state.json")).unwrap();
    }

    #[test]
    fn needs_migration_true_when_state_exists_no_intents() {
        let dir = test_dir("needs_true");
        write_old_state(&dir, &YokeState::new());

        assert!(needs_migration(&dir));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn needs_migration_false_when_intents_exist() {
        let dir = test_dir("needs_false_intents");
        write_old_state(&dir, &YokeState::new());
        std::fs::create_dir_all(dir.join(".yoke/intents")).unwrap();

        assert!(!needs_migration(&dir));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn needs_migration_false_when_no_state() {
        let dir = test_dir("needs_false_no_state");
        std::fs::create_dir_all(dir.join(".yoke")).unwrap();

        assert!(!needs_migration(&dir));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn full_migration_moves_docs() {
        let dir = test_dir("full_migration");
        let state = YokeState {
            spec_status: StageStatus::Complete,
            plan_status: StageStatus::Complete,
            phases: vec![PhaseState {
                number: 1,
                title: "Setup".to_string(),
                status: PhaseStatus::Completed,
                current_step: None,
                cost_usd: 0.25,
                step_costs: Vec::new(),
                pre_phase_commit: None,
                started_at: None,
                completed_at: None,
            }],
            total_cost_usd: 1.50,
            spec_cost_usd: 0.50,
            plan_cost_usd: 0.30,
            ..YokeState::new()
        };
        write_old_state(&dir, &state);

        let docs = dir.join("docs");
        std::fs::create_dir_all(docs.join("phases")).unwrap();
        std::fs::create_dir_all(docs.join("research")).unwrap();
        std::fs::create_dir_all(docs.join("plans")).unwrap();
        std::fs::create_dir_all(docs.join("handoffs")).unwrap();
        std::fs::create_dir_all(docs.join("extracts")).unwrap();
        std::fs::create_dir_all(docs.join("summaries")).unwrap();

        std::fs::write(docs.join("product-spec.md"), "product spec").unwrap();
        std::fs::write(docs.join("technical-spec.md"), "technical spec").unwrap();
        std::fs::write(docs.join("plan.md"), "the plan").unwrap();
        std::fs::write(docs.join("phases/phase-1.md"), "phase 1").unwrap();
        std::fs::write(docs.join("research/notes.md"), "research notes").unwrap();
        std::fs::write(docs.join("plans/plan-1.md"), "plan detail").unwrap();
        std::fs::write(docs.join("handoffs/handoff-1.md"), "handoff").unwrap();
        std::fs::write(docs.join("extracts/extract.md"), "transient").unwrap();
        std::fs::write(docs.join("summaries/summary.md"), "transient").unwrap();

        let config = minimal_config("test-project");
        let intent = migrate(&dir, &config).unwrap();

        assert_eq!(intent.id, "i-001");
        assert_eq!(intent.title, "initial build");
        assert_eq!(intent.description, "test-project initial build");
        assert_eq!(intent.classification, Classification::Build);
        assert_eq!(intent.depth, Depth::Full);
        assert_eq!(intent.total_cost_usd, 1.50);
        assert_eq!(intent.spec_cost_usd, 0.50);
        assert_eq!(intent.plan_cost_usd, 0.30);

        let specs_dir = dir.join(".yoke/specs");
        assert_eq!(
            std::fs::read_to_string(specs_dir.join("product.md")).unwrap(),
            "product spec"
        );
        assert_eq!(
            std::fs::read_to_string(specs_dir.join("technical.md")).unwrap(),
            "technical spec"
        );

        let intent_dir = dir.join(".yoke/intents/i-001-initial-build");
        assert_eq!(
            std::fs::read_to_string(intent_dir.join("plan.md")).unwrap(),
            "the plan"
        );
        assert_eq!(
            std::fs::read_to_string(intent_dir.join("phases/phase-1.md")).unwrap(),
            "phase 1"
        );
        assert_eq!(
            std::fs::read_to_string(intent_dir.join("research/notes.md")).unwrap(),
            "research notes"
        );
        assert_eq!(
            std::fs::read_to_string(intent_dir.join("plans/plan-1.md")).unwrap(),
            "plan detail"
        );
        assert_eq!(
            std::fs::read_to_string(intent_dir.join("handoffs/handoff-1.md")).unwrap(),
            "handoff"
        );

        assert!(intent_dir.join("intent.json").exists());
        assert!(!docs.exists());
        assert!(!dir.join(".yoke/state.json").exists());
        assert!(dir.join(".yoke/state.json.migrated").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_status_completed_when_all_phases_done() {
        let dir = test_dir("status_completed");
        let state = YokeState {
            spec_status: StageStatus::Complete,
            plan_status: StageStatus::Complete,
            phases: vec![
                PhaseState {
                    number: 1,
                    title: "First".to_string(),
                    status: PhaseStatus::Completed,
                    current_step: None,
                    cost_usd: 0.0,
                    step_costs: Vec::new(),
                    pre_phase_commit: None,
                    started_at: None,
                    completed_at: None,
                },
                PhaseState {
                    number: 2,
                    title: "Second".to_string(),
                    status: PhaseStatus::Completed,
                    current_step: None,
                    cost_usd: 0.0,
                    step_costs: Vec::new(),
                    pre_phase_commit: None,
                    started_at: None,
                    completed_at: None,
                },
            ],
            ..YokeState::new()
        };
        write_old_state(&dir, &state);

        let config = minimal_config("test");
        let intent = migrate(&dir, &config).unwrap();

        assert_eq!(intent.status, IntentStatus::Completed);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_status_in_progress_when_phase_active() {
        let dir = test_dir("status_in_progress");
        let state = YokeState {
            phases: vec![
                PhaseState {
                    number: 1,
                    title: "Done".to_string(),
                    status: PhaseStatus::Completed,
                    current_step: None,
                    cost_usd: 0.0,
                    step_costs: Vec::new(),
                    pre_phase_commit: None,
                    started_at: None,
                    completed_at: None,
                },
                PhaseState {
                    number: 2,
                    title: "Active".to_string(),
                    status: PhaseStatus::InProgress,
                    current_step: None,
                    cost_usd: 0.0,
                    step_costs: Vec::new(),
                    pre_phase_commit: None,
                    started_at: None,
                    completed_at: None,
                },
            ],
            ..YokeState::new()
        };
        write_old_state(&dir, &state);

        let config = minimal_config("test");
        let intent = migrate(&dir, &config).unwrap();

        assert_eq!(intent.status, IntentStatus::InProgress);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_creates_all_directories() {
        let dir = test_dir("creates_dirs");
        write_old_state(&dir, &YokeState::new());

        let config = minimal_config("test");
        migrate(&dir, &config).unwrap();

        assert!(dir.join(".yoke/specs").is_dir());
        let intent_dir = dir.join(".yoke/intents/i-001-initial-build");
        assert!(intent_dir.join("phases").is_dir());
        assert!(intent_dir.join("research").is_dir());
        assert!(intent_dir.join("plans").is_dir());
        assert!(intent_dir.join("handoffs").is_dir());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_renames_state_json() {
        let dir = test_dir("renames_state");
        write_old_state(&dir, &YokeState::new());

        let config = minimal_config("test");
        migrate(&dir, &config).unwrap();

        assert!(!dir.join(".yoke/state.json").exists());
        assert!(dir.join(".yoke/state.json.migrated").exists());

        let migrated: YokeState = serde_json::from_str(
            &std::fs::read_to_string(dir.join(".yoke/state.json.migrated")).unwrap(),
        )
        .unwrap();
        assert_eq!(migrated.spec_status, StageStatus::Pending);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn error_on_corrupt_state_json() {
        let dir = test_dir("corrupt_state");
        let yoke_dir = dir.join(".yoke");
        std::fs::create_dir_all(&yoke_dir).unwrap();
        std::fs::write(yoke_dir.join("state.json"), "not valid json").unwrap();

        let config = minimal_config("test");
        let result = migrate(&dir, &config);

        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
