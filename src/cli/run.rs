use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::claude;
use crate::config::{InteractionMode, YokeConfig};
use crate::git;
use crate::intent::migrate;
use crate::intent::store::IntentStore;
use crate::intent::{IntentState, IntentStatus};
use crate::state::PhaseStatus;
use crate::workflow;
use crate::workflow::IntentContext;
use crate::workflow::phase::parse_step_name;
use crate::workflow::pipeline;

pub async fn run(
    project_dir: &Path,
    intent_id: Option<String>,
    phase_number: Option<usize>,
    from: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let config_path = project_dir.join(".yoke/config.toml");
    let config = YokeConfig::load(&config_path)
        .context("could not load config; have you run `yoke init`?")?;

    if migrate::needs_migration(project_dir) {
        migrate::migrate(project_dir, &config)?;
    }

    let store = IntentStore::new(project_dir);

    let mut intent = match &intent_id {
        Some(id) => store.load(id)?,
        None => find_target_intent(&store)?,
    };

    if !intent.blocked_by.is_empty() {
        let all_intents = store.list()?;
        let mut blocking = Vec::new();
        for blocker_id in &intent.blocked_by {
            if let Some(blocker) = all_intents.iter().find(|i| &i.id == blocker_id)
                && blocker.status != IntentStatus::Completed
            {
                blocking.push(blocker_id.clone());
            }
        }
        if !blocking.is_empty() {
            bail!(
                "intent {} is blocked by: {}. Complete those intents first.",
                intent.id,
                blocking.join(", ")
            );
        }
    }

    if !dry_run {
        claude::verify_available().await?;
    }

    let intent_dir = store.intent_dir(&intent);
    let specs_dir = project_dir.join(".yoke/specs");

    let work_dir = resolve_work_dir(project_dir, &mut intent, &config, &store).await?;

    let loader = workflow::prompt_loader(project_dir);
    let system_prompt = workflow::build_system_prompt(&config, &loader)?;

    let mut ctx = IntentContext {
        project_dir,
        work_dir,
        intent_dir,
        specs_dir,
        intent: &mut intent,
        config: &config,
        store: &store,
        loader,
        system_prompt,
        dry_run,
    };

    if let Some(step_name) = &from {
        let target = phase_number
            .or_else(|| pipeline::next_pending_phase_number(&ctx))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "--from requires a target phase. Use --phase or have a pending phase."
                )
            })?;
        let step = parse_step_name(step_name)?;
        let phase = ctx
            .intent
            .phases
            .iter_mut()
            .find(|p| p.number == target)
            .ok_or_else(|| anyhow::anyhow!("phase {target} not found in intent state"))?;
        phase.status = PhaseStatus::InProgress;
        phase.current_step = Some(step);
        ctx.save_intent()?;
    }

    if ctx.intent.status == IntentStatus::Pending {
        ctx.intent.status = IntentStatus::InProgress;
        ctx.intent.started_at = Some(chrono::Utc::now());
        ctx.save_intent()?;
    }

    pipeline::run_intent(&mut ctx, phase_number).await?;

    let auto_advance = phase_number.is_none() && config.interaction != InteractionMode::Milestones;
    if auto_advance && pipeline::all_phases_complete(&ctx) {
        ctx.intent.status = IntentStatus::Completed;
        ctx.intent.completed_at = Some(chrono::Utc::now());
        ctx.save_intent()?;
    }

    Ok(())
}

async fn resolve_work_dir(
    project_dir: &Path,
    intent: &mut IntentState,
    config: &YokeConfig,
    store: &IntentStore,
) -> Result<PathBuf> {
    if intent.branch.is_none() {
        return Ok(project_dir.to_path_buf());
    }
    let branch = intent.branch.clone().unwrap();

    if let Some(ref wt_path) = intent.worktree_path {
        let p = PathBuf::from(wt_path);
        if p.exists() {
            return Ok(p);
        }
        let wt = git::create_worktree(project_dir, &p, &branch).await?;
        return Ok(wt);
    }

    let active = store
        .list()?
        .into_iter()
        .any(|i| i.id != intent.id && i.status == IntentStatus::InProgress);

    if active {
        let wt_dir = git::resolve_worktree_dir(config, intent, project_dir);
        let wt = git::create_worktree(project_dir, &wt_dir, &branch).await?;
        intent.worktree_path = Some(wt.display().to_string());
        let intent_dir = store.intent_dir(intent);
        intent.save(&intent_dir.join("intent.json"))?;
        Ok(wt)
    } else {
        git::checkout_branch(project_dir, &branch).await?;
        Ok(project_dir.to_path_buf())
    }
}

fn find_target_intent(store: &IntentStore) -> Result<crate::intent::IntentState> {
    let intents = store.list()?;

    for intent in &intents {
        if intent.status == IntentStatus::InProgress {
            return store.load(&intent.id);
        }
    }
    for intent in &intents {
        if intent.status == IntentStatus::Pending {
            return store.load(&intent.id);
        }
    }
    for intent in &intents {
        if intent.status == IntentStatus::Failed {
            return store.load(&intent.id);
        }
    }

    bail!("no pending, in-progress, or failed intents found. Create one with `yoke new`.")
}
