use std::path::Path;

use anyhow::{Result, bail};

use crate::config::YokeConfig;
use crate::state::YokeState;

pub fn run(project_dir: &Path) -> Result<()> {
    let yoke_dir = project_dir.join(".yoke");

    if yoke_dir.exists() {
        bail!(".yoke directory already exists at {}", yoke_dir.display());
    }

    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed");

    std::fs::create_dir_all(&yoke_dir)?;
    std::fs::create_dir_all(yoke_dir.join("prompts"))?;

    let config_path = yoke_dir.join("config.toml");
    std::fs::write(&config_path, YokeConfig::default_toml(project_name))?;

    let state_path = yoke_dir.join("state.json");
    let state = YokeState::new();
    state.save(&state_path)?;

    println!("initialized .yoke in {}", project_dir.display());

    Ok(())
}
