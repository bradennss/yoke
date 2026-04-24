use anyhow::Result;

use crate::config::InteractionMode;
use crate::intent::Depth;
use crate::state::{PhaseState, PhaseStatus, StageStatus};
use crate::workflow;
use crate::workflow::IntentContext;

pub async fn run_intent(ctx: &mut IntentContext<'_>, target_phase: Option<usize>) -> Result<()> {
    match ctx.intent.depth {
        Depth::Full => run_full(ctx, target_phase).await,
        Depth::Light => run_light(ctx, target_phase).await,
        Depth::Minimal => run_minimal(ctx).await,
    }
}

async fn run_full(ctx: &mut IntentContext<'_>, target_phase: Option<usize>) -> Result<()> {
    if ctx.intent.spec_status != StageStatus::Complete {
        let description = ctx.intent.description.clone();
        workflow::spec::run_spec(ctx, &description).await?;
    }

    if ctx.intent.plan_status != StageStatus::Complete {
        workflow::plan::run_plan(ctx).await?;
    }

    run_phases(ctx, target_phase).await
}

async fn run_light(ctx: &mut IntentContext<'_>, target_phase: Option<usize>) -> Result<()> {
    if ctx.intent.plan_status != StageStatus::Complete {
        workflow::plan::run_plan(ctx).await?;
    }

    run_phases(ctx, target_phase).await
}

async fn run_minimal(ctx: &mut IntentContext<'_>) -> Result<()> {
    if ctx.intent.phases.is_empty() {
        let phase = PhaseState {
            number: 1,
            title: ctx.intent.title.clone(),
            status: PhaseStatus::Pending,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: None,
        };
        ctx.intent.phases.push(phase);
        ctx.save_intent()?;
    }

    workflow::phase::run_phase(ctx, 1).await
}

async fn run_phases(ctx: &mut IntentContext<'_>, target_phase: Option<usize>) -> Result<()> {
    if let Some(phase_num) = target_phase {
        workflow::phase::run_phase(ctx, phase_num).await?;
        return Ok(());
    }

    let first_target = next_pending_phase_number(ctx);
    if let Some(target) = first_target {
        workflow::phase::run_phase(ctx, target).await?;
    }

    let auto_advance = ctx.config.interaction != InteractionMode::Milestones;
    if auto_advance {
        while let Some(next) = next_pending_phase_number(ctx) {
            workflow::phase::run_phase(ctx, next).await?;
        }
    }

    Ok(())
}

pub fn next_pending_phase_number(ctx: &IntentContext<'_>) -> Option<usize> {
    for phase in &ctx.intent.phases {
        if phase.status == PhaseStatus::InProgress {
            return Some(phase.number);
        }
    }
    for phase in &ctx.intent.phases {
        if phase.status == PhaseStatus::Pending || phase.status == PhaseStatus::Failed {
            return Some(phase.number);
        }
    }
    None
}

pub fn all_phases_complete(ctx: &IntentContext<'_>) -> bool {
    !ctx.intent.phases.is_empty()
        && ctx
            .intent
            .phases
            .iter()
            .all(|p| p.status == PhaseStatus::Completed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{Classification, Depth, IntentState};
    use crate::state::PhaseStep;
    use chrono::Utc;

    fn make_intent(depth: Depth) -> IntentState {
        IntentState::new(
            1,
            "Test".to_string(),
            "Test description".to_string(),
            Classification::Build,
            depth,
        )
    }

    #[test]
    fn all_phases_complete_empty_returns_false() {
        let mut intent = make_intent(Depth::Full);
        let config: crate::config::YokeConfig =
            toml::from_str("[project]\nname = \"test\"").unwrap();
        let store = crate::intent::store::IntentStore::new(std::path::Path::new("/tmp"));
        let loader = crate::prompts::PromptLoader::new(None);
        let ctx = IntentContext {
            project_dir: std::path::Path::new("/tmp"),
            work_dir: std::path::PathBuf::from("/tmp"),
            intent_dir: std::path::PathBuf::from("/tmp/intent"),
            specs_dir: std::path::PathBuf::from("/tmp/specs"),
            intent: &mut intent,
            config: &config,
            store: &store,
            loader,
            system_prompt: String::new(),
            dry_run: false,
        };
        assert!(!all_phases_complete(&ctx));
    }

    #[test]
    fn all_phases_complete_with_completed_phases() {
        let mut intent = make_intent(Depth::Full);
        intent.phases.push(PhaseState {
            number: 1,
            title: "Phase 1".to_string(),
            status: PhaseStatus::Completed,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: Some(Utc::now()),
        });
        let config: crate::config::YokeConfig =
            toml::from_str("[project]\nname = \"test\"").unwrap();
        let store = crate::intent::store::IntentStore::new(std::path::Path::new("/tmp"));
        let loader = crate::prompts::PromptLoader::new(None);
        let ctx = IntentContext {
            project_dir: std::path::Path::new("/tmp"),
            work_dir: std::path::PathBuf::from("/tmp"),
            intent_dir: std::path::PathBuf::from("/tmp/intent"),
            specs_dir: std::path::PathBuf::from("/tmp/specs"),
            intent: &mut intent,
            config: &config,
            store: &store,
            loader,
            system_prompt: String::new(),
            dry_run: false,
        };
        assert!(all_phases_complete(&ctx));
    }

    #[test]
    fn all_phases_complete_with_pending_phase() {
        let mut intent = make_intent(Depth::Full);
        intent.phases.push(PhaseState {
            number: 1,
            title: "Phase 1".to_string(),
            status: PhaseStatus::Completed,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: Some(Utc::now()),
        });
        intent.phases.push(PhaseState {
            number: 2,
            title: "Phase 2".to_string(),
            status: PhaseStatus::Pending,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: None,
        });
        let config: crate::config::YokeConfig =
            toml::from_str("[project]\nname = \"test\"").unwrap();
        let store = crate::intent::store::IntentStore::new(std::path::Path::new("/tmp"));
        let loader = crate::prompts::PromptLoader::new(None);
        let ctx = IntentContext {
            project_dir: std::path::Path::new("/tmp"),
            work_dir: std::path::PathBuf::from("/tmp"),
            intent_dir: std::path::PathBuf::from("/tmp/intent"),
            specs_dir: std::path::PathBuf::from("/tmp/specs"),
            intent: &mut intent,
            config: &config,
            store: &store,
            loader,
            system_prompt: String::new(),
            dry_run: false,
        };
        assert!(!all_phases_complete(&ctx));
    }

    #[test]
    fn next_pending_picks_in_progress_first() {
        let mut intent = make_intent(Depth::Full);
        intent.phases.push(PhaseState {
            number: 1,
            title: "Phase 1".to_string(),
            status: PhaseStatus::Completed,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: Some(Utc::now()),
        });
        intent.phases.push(PhaseState {
            number: 2,
            title: "Phase 2".to_string(),
            status: PhaseStatus::InProgress,
            current_step: Some(PhaseStep::Execution),
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: Some(Utc::now()),
            completed_at: None,
        });
        intent.phases.push(PhaseState {
            number: 3,
            title: "Phase 3".to_string(),
            status: PhaseStatus::Pending,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: None,
        });
        let config: crate::config::YokeConfig =
            toml::from_str("[project]\nname = \"test\"").unwrap();
        let store = crate::intent::store::IntentStore::new(std::path::Path::new("/tmp"));
        let loader = crate::prompts::PromptLoader::new(None);
        let ctx = IntentContext {
            project_dir: std::path::Path::new("/tmp"),
            work_dir: std::path::PathBuf::from("/tmp"),
            intent_dir: std::path::PathBuf::from("/tmp/intent"),
            specs_dir: std::path::PathBuf::from("/tmp/specs"),
            intent: &mut intent,
            config: &config,
            store: &store,
            loader,
            system_prompt: String::new(),
            dry_run: false,
        };
        assert_eq!(next_pending_phase_number(&ctx), Some(2));
    }
}
