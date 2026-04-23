use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::claude;
use crate::config::YokeConfig;
use crate::state::YokeState;
use crate::workflow;

pub async fn run(
    project_dir: &Path,
    description: Option<String>,
    file: Option<PathBuf>,
    dry_run: bool,
) -> Result<()> {
    let description = resolve_description(description, file)?;

    let config_path = project_dir.join(".yoke/config.toml");
    let config = YokeConfig::load(&config_path)
        .context("could not load config; have you run `yoke init`?")?;

    let state_path = project_dir.join(".yoke/state.json");
    let mut state =
        YokeState::load(&state_path).context("could not load state; have you run `yoke init`?")?;

    if !dry_run {
        claude::verify_available().await?;
    }

    workflow::spec::run_spec(project_dir, &description, &config, &mut state, dry_run).await?;

    Ok(())
}

fn resolve_description(description: Option<String>, file: Option<PathBuf>) -> Result<String> {
    if let Some(desc) = description {
        return Ok(desc);
    }

    if let Some(path) = file {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading description file {}", path.display()))?;
        return Ok(content);
    }

    if atty::is(atty::Stream::Stdin) {
        bail!(
            "no description provided. Pass a description as an argument, use --file, or pipe via stdin."
        );
    }

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("reading description from stdin")?;

    if input.trim().is_empty() {
        bail!("received empty description from stdin");
    }

    Ok(input)
}
