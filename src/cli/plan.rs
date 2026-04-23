use std::path::Path;

use anyhow::{Context, Result};

use crate::claude;
use crate::config::YokeConfig;
use crate::state::YokeState;
use crate::workflow;

pub async fn run(project_dir: &Path, dry_run: bool) -> Result<()> {
    let config_path = project_dir.join(".yoke/config.toml");
    let config = YokeConfig::load(&config_path)
        .context("could not load config; have you run `yoke init`?")?;

    let state_path = project_dir.join(".yoke/state.json");
    let mut state =
        YokeState::load(&state_path).context("could not load state; have you run `yoke init`?")?;

    if !dry_run {
        claude::verify_available().await?;
    }

    workflow::plan::run_plan(project_dir, &config, &mut state, dry_run).await?;

    Ok(())
}
