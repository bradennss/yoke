use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::YokeConfig;
use crate::git;
use crate::intent::store::IntentStore;
use crate::intent::{Classification, Depth, IntentState};
use crate::workflow::classify::{classify_intent, depth_for_classification};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    project_dir: &Path,
    description: Option<String>,
    file: Option<PathBuf>,
    class: Option<String>,
    depth_override: Option<String>,
    parent: Option<String>,
    blocked_by: Option<Vec<String>>,
    yes: bool,
    dry_run: bool,
) -> Result<()> {
    let description = resolve_description(description, file)?;

    let config_path = project_dir.join(".yoke/config.toml");
    let config = YokeConfig::load(&config_path)
        .context("could not load config; have you run `yoke init`?")?;

    let store = IntentStore::new(project_dir);

    let (classification, depth) = if let Some(class_str) = class {
        let classification = parse_classification(&class_str)?;
        let depth = if let Some(d) = &depth_override {
            parse_depth(d)?
        } else {
            depth_for_classification(classification)
        };
        (classification, depth)
    } else {
        let result = classify_intent(&config, &description, dry_run).await?;

        if !yes {
            eprintln!(
                "Classification: {} ({} depth)",
                result.classification, result.depth
            );
            eprintln!("Reasoning: {}", result.reasoning);
            eprintln!("Accept? [Y/n]");

            let mut buf = [0u8; 1];
            let accepted = match std::io::stdin().read(&mut buf) {
                Ok(0) => true,
                Ok(_) => matches!(buf[0], b'y' | b'Y' | b'\n' | b'\r'),
                Err(_) => true,
            };

            if !accepted {
                bail!("classification rejected by user");
            }
        }

        let depth = if let Some(d) = &depth_override {
            parse_depth(d)?
        } else {
            result.depth
        };
        (result.classification, depth)
    };

    let next_num = store.next_number()?;
    let is_first_build = next_num == 1 && classification == Classification::Build;

    if depth == Depth::Full && !is_first_build {
        let product_spec = project_dir.join(".yoke/specs/product.md");
        if !product_spec.exists() {
            bail!(
                "no project specs found. Run `yoke discover` first to generate specs from your existing codebase, or use `--depth light` to skip specs."
            );
        }

        if let Some(active) = store.active_full_depth()? {
            bail!(
                "intent {} is already active at Full depth. Only one Full-depth intent may be in progress at a time.",
                active.id
            );
        }
    }

    let title = derive_title(&description);

    let mut intent = IntentState::new(next_num, title.clone(), description, classification, depth);

    if let Some(p) = parent {
        intent.parent = Some(p);
    }
    if let Some(blockers) = blocked_by {
        intent.blocked_by = blockers;
    }

    store.create(&intent)?;
    store.ensure_dirs(&intent)?;

    if !is_first_build {
        let branch_name = git::intent_branch_name(&config.git, &intent);
        git::create_branch(project_dir, &branch_name).await?;
        intent.branch = Some(branch_name.clone());
        let intent_json = store.intent_dir(&intent).join("intent.json");
        intent.save(&intent_json)?;
        eprintln!("Created branch {branch_name}");
    }

    println!(
        "Created intent {} ({}, {}, {})",
        intent.id, title, classification, depth
    );

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

fn parse_classification(s: &str) -> Result<Classification> {
    match s.to_lowercase().as_str() {
        "build" => Ok(Classification::Build),
        "feature" => Ok(Classification::Feature),
        "fix" => Ok(Classification::Fix),
        "refactor" => Ok(Classification::Refactor),
        "maintenance" => Ok(Classification::Maintenance),
        _ => bail!(
            "unknown classification: {s}. Valid values: build, feature, fix, refactor, maintenance"
        ),
    }
}

fn parse_depth(s: &str) -> Result<Depth> {
    match s.to_lowercase().as_str() {
        "full" => Ok(Depth::Full),
        "light" => Ok(Depth::Light),
        "minimal" => Ok(Depth::Minimal),
        _ => bail!("unknown depth: {s}. Valid values: full, light, minimal"),
    }
}

fn derive_title(description: &str) -> String {
    let first_line = description.lines().next().unwrap_or(description);
    let trimmed = first_line.trim();
    if trimmed.len() <= 60 {
        trimmed.to_string()
    } else {
        let mut title: String = trimmed.chars().take(57).collect();
        title.push_str("...");
        title
    }
}
