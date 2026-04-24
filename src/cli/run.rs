use std::path::Path;

use anyhow::{Context, Result};

use crate::claude;
use crate::config::{InteractionMode, YokeConfig};
use crate::state::{PhaseStatus, YokeState};
use crate::workflow;
use crate::workflow::phase::parse_step_name;

pub async fn run(
    project_dir: &Path,
    phase_number: Option<usize>,
    from: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let config_path = project_dir.join(".yoke/config.toml");
    let config = YokeConfig::load(&config_path)
        .context("could not load config; have you run `yoke init`?")?;

    let state_path = project_dir.join(".yoke/state.json");
    let mut state =
        YokeState::load(&state_path).context("could not load state; have you run `yoke init`?")?;

    if !dry_run {
        claude::verify_available().await?;
    }

    let target_phase = match phase_number {
        Some(n) => n,
        None => next_pending_phase(&state).ok_or_else(|| {
            anyhow::anyhow!("no pending or failed phases found. All phases are complete.")
        })?,
    };

    if let Some(step_name) = &from {
        let step = parse_step_name(step_name)?;
        let phase = state
            .phases
            .iter_mut()
            .find(|p| p.number == target_phase)
            .ok_or_else(|| anyhow::anyhow!("phase {target_phase} not found in state"))?;
        phase.status = PhaseStatus::InProgress;
        phase.current_step = Some(step);
        state.save(&state_path)?;
    }

    workflow::phase::run_phase(project_dir, target_phase, &config, &mut state, dry_run).await?;

    let auto_advance = phase_number.is_none() && config.interaction != InteractionMode::Milestones;
    if auto_advance {
        while let Some(next) = next_pending_phase(&state) {
            workflow::phase::run_phase(project_dir, next, &config, &mut state, dry_run).await?;
        }
    }

    Ok(())
}

fn next_pending_phase(state: &YokeState) -> Option<usize> {
    for phase in &state.phases {
        if phase.status == PhaseStatus::InProgress {
            return Some(phase.number);
        }
    }
    for phase in &state.phases {
        if phase.status == PhaseStatus::Pending || phase.status == PhaseStatus::Failed {
            return Some(phase.number);
        }
    }
    None
}
