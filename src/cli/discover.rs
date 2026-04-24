use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::claude;
use crate::config::YokeConfig;
use crate::output::StreamDisplay;
use crate::template;
use crate::workflow;
use crate::workflow::context::ContextBuilder;
use crate::workflow::review::{ReviewParams, run_review_loop};

const GENERATION_TOOLS: &str = "Read,Write,Edit,Glob,Grep,Bash";
const REVIEW_TOOLS: &str = "Read,Glob,Grep";

pub async fn run(project_dir: &Path, dry_run: bool) -> Result<()> {
    let specs_dir = project_dir.join(".yoke/specs");
    let product_spec_path = specs_dir.join("product.md");

    if product_spec_path.exists() {
        bail!("specs already exist. To regenerate, delete .yoke/specs/ and run again.");
    }

    let config_path = project_dir.join(".yoke/config.toml");
    let config = YokeConfig::load(&config_path)
        .context("could not load config; have you run `yoke init`?")?;

    if !dry_run {
        claude::verify_available().await?;
    }

    std::fs::create_dir_all(&specs_dir).context("creating specs directory")?;

    let loader = workflow::prompt_loader(project_dir);
    let system_prompt = workflow::build_system_prompt(&config, &loader)?;

    let product_template = loader.load("discover_product")?;
    let product_prompt = {
        let mut ctx = ContextBuilder::new();
        ctx.add_content("project_name", &config.project.name);
        template::replace_vars(
            &ctx.apply(&product_template),
            &[
                ("project_name", &config.project.name),
                ("target_file", &product_spec_path.display().to_string()),
            ],
        )
    };

    crate::output::print_step("Discovering product specification from codebase");
    let mut display = StreamDisplay::new();
    workflow::invoke_sub_agent(
        &product_prompt,
        &config.discover.model,
        config.discover.effort,
        Some(GENERATION_TOOLS),
        Some(&system_prompt),
        Some(project_dir),
        &mut display,
        &config.retry,
        dry_run,
    )
    .await?;

    if !dry_run && !product_spec_path.exists() {
        bail!("sub agent did not create {}", product_spec_path.display());
    }

    let technical_spec_path = specs_dir.join("technical.md");
    let tech_template = loader.load("discover_technical")?;
    let tech_prompt = {
        let mut ctx = ContextBuilder::new();
        if product_spec_path.exists() {
            ctx.add_file("product spec", &product_spec_path)?;
        }
        template::replace_vars(
            &ctx.apply(&tech_template),
            &[
                ("project_name", &config.project.name),
                ("target_file", &technical_spec_path.display().to_string()),
            ],
        )
    };

    crate::output::print_step("Discovering technical specification from codebase");
    let mut display = StreamDisplay::new();
    workflow::invoke_sub_agent(
        &tech_prompt,
        &config.discover.model,
        config.discover.effort,
        Some(GENERATION_TOOLS),
        Some(&system_prompt),
        Some(project_dir),
        &mut display,
        &config.retry,
        dry_run,
    )
    .await?;

    if !dry_run && !technical_spec_path.exists() {
        bail!("sub agent did not create {}", technical_spec_path.display());
    }

    let review_template = loader.load("discover_review")?;

    let product_review_prompt = template::replace_vars(
        &review_template,
        &[
            ("project_name", &config.project.name),
            ("target_file", &product_spec_path.display().to_string()),
        ],
    );
    let product_path_clone = product_spec_path.clone();
    let mut product_review_params = ReviewParams {
        config: &config,
        prompt_template: &product_review_prompt,
        model: &config.discover.review_model,
        effort: config.discover.review_effort,
        max_iterations: config.discover.max_review_iterations,
        tools: Some(REVIEW_TOOLS),
        system_prompt: Some(&system_prompt),
        cwd: Some(project_dir),
        dry_run,
        prior_findings: None,
    };
    let product_context_fn = || {
        let path = product_path_clone.clone();
        async move {
            let mut cb = ContextBuilder::new();
            cb.add_file("product spec", &path)?;
            Ok(cb)
        }
    };
    run_review_loop(
        &mut product_review_params,
        config.discover.review_effort,
        1,
        "Reviewing product spec",
        &product_context_fn,
        |_iteration, _cost| Ok(()),
    )
    .await?;

    let tech_review_prompt = template::replace_vars(
        &review_template,
        &[
            ("project_name", &config.project.name),
            ("target_file", &technical_spec_path.display().to_string()),
        ],
    );
    let tech_path_clone = technical_spec_path.clone();
    let product_path_clone2 = product_spec_path.clone();
    let mut tech_review_params = ReviewParams {
        config: &config,
        prompt_template: &tech_review_prompt,
        model: &config.discover.review_model,
        effort: config.discover.review_effort,
        max_iterations: config.discover.max_review_iterations,
        tools: Some(REVIEW_TOOLS),
        system_prompt: Some(&system_prompt),
        cwd: Some(project_dir),
        dry_run,
        prior_findings: None,
    };
    let tech_context_fn = || {
        let tech = tech_path_clone.clone();
        let prod = product_path_clone2.clone();
        async move {
            let mut cb = ContextBuilder::new();
            cb.add_file("technical spec", &tech)?;
            cb.add_file("product spec", &prod)?;
            Ok(cb)
        }
    };
    run_review_loop(
        &mut tech_review_params,
        config.discover.review_effort,
        1,
        "Reviewing technical spec",
        &tech_context_fn,
        |_iteration, _cost| Ok(()),
    )
    .await?;

    println!(
        "Project specs generated at .yoke/specs/. Review them, then create intents with `yoke new`."
    );

    Ok(())
}
