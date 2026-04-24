use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::config::{InteractionMode, YokeConfig};
use crate::output::StreamDisplay;
use crate::state::{SpecStep, StageStatus, YokeState, spec_step_ordinal};
use crate::template;
use crate::workflow::cleanup::SingleFileGuard;
use crate::workflow::context::ContextBuilder;
use crate::workflow::review::{ReviewParams, Verdict, run_review_iteration};
use crate::workflow::{build_system_prompt, invoke_sub_agent, prompt_loader};

const GENERATION_TOOLS: &str = "Read,Write,Edit,Glob,Grep,Bash";
const REVIEW_TOOLS: &str = "Read,Write,Edit,Glob,Grep";

pub async fn run_spec(
    project_dir: &Path,
    description: &str,
    config: &YokeConfig,
    state: &mut YokeState,
    dry_run: bool,
) -> Result<()> {
    let docs_dir = project_dir.join("docs");
    std::fs::create_dir_all(&docs_dir).context("creating docs directory")?;

    let loader = prompt_loader(project_dir);
    let system_prompt = build_system_prompt(config, &loader)?;
    let state_path = project_dir.join(".yoke/state.json");

    let starting_step = state
        .spec_step
        .clone()
        .unwrap_or(SpecStep::ProductSpecGeneration);
    let start_ord = spec_step_ordinal(&starting_step);

    state.spec_status = StageStatus::InProgress;
    state.save(&state_path)?;

    let product_spec_path = docs_dir.join("product-spec.md");

    if start_ord <= spec_step_ordinal(&SpecStep::ProductSpecGeneration) {
        crate::output::print_step("Generating product spec from project description");
        state.spec_step = Some(SpecStep::ProductSpecGeneration);
        state.save(&state_path)?;

        let mut cleanup = SingleFileGuard::new(&product_spec_path);

        let product_template = loader.load("spec_product")?;
        let mut ctx = ContextBuilder::new();
        ctx.add_content("description", description);
        let prompt = template::replace_vars(
            &ctx.apply(&product_template),
            &[
                ("project_name", &config.project.name),
                ("target_file", "docs/product-spec.md"),
            ],
        );

        let mut display = StreamDisplay::new();
        display.set_context_stats(ctx.total_tokens(), ctx.block_stats().len());
        let result = invoke_sub_agent(
            &prompt,
            &config.models.spec,
            config.effort.spec,
            Some(GENERATION_TOOLS),
            Some(system_prompt.as_str()),
            Some(project_dir),
            &mut display,
            &config.retry,
            dry_run,
        )
        .await?;

        if dry_run {
            return Ok(());
        }

        state.spec_cost_usd += result.cost_usd;
        state.total_cost_usd += result.cost_usd;
        state.save(&state_path)?;

        if !product_spec_path.exists() {
            bail!("sub agent did not create docs/product-spec.md");
        }

        cleanup.defuse();
    }

    if start_ord <= spec_step_ordinal(&SpecStep::ProductSpecReview { iteration: 0 }) {
        let starting_iteration = match &starting_step {
            SpecStep::ProductSpecReview { iteration } if *iteration > 0 => *iteration + 1,
            _ => 1,
        };

        state.spec_step = Some(SpecStep::ProductSpecReview {
            iteration: starting_iteration.saturating_sub(1),
        });
        state.save(&state_path)?;

        let review_template = loader.load("spec_review")?;
        let review_prompt = template::replace_vars(
            &review_template,
            &[
                ("project_name", &config.project.name),
                ("target_file", "docs/product-spec.md"),
            ],
        );

        let product_spec_path_clone = product_spec_path.clone();
        let review_params = ReviewParams {
            config,
            prompt_template: &review_prompt,
            model: &config.models.review,
            effort: config.effort.review,
            max_iterations: config.review.max_iterations,
            tools: Some(REVIEW_TOOLS),
            system_prompt: Some(system_prompt.as_str()),
            cwd: Some(project_dir),
            dry_run,
        };
        let context_fn = || {
            let path = product_spec_path_clone.clone();
            async move {
                let mut cb = ContextBuilder::new();
                cb.add_file("product-spec.md", &path)?;
                Ok(cb)
            }
        };

        let max = config.review.max_iterations;
        let mut display = StreamDisplay::new();
        let mut converged = false;
        for iteration in starting_iteration..=max {
            crate::output::print_step(&format!(
                "Reviewing product spec, iteration {iteration}/{max}"
            ));
            let iter_result =
                run_review_iteration(&review_params, &context_fn, &mut display).await?;

            state.spec_cost_usd += iter_result.cost_usd;
            state.total_cost_usd += iter_result.cost_usd;
            state.spec_step = Some(SpecStep::ProductSpecReview { iteration });
            state.save(&state_path)?;

            if iter_result.verdict == Verdict::Clean {
                converged = true;
                break;
            }
        }

        if !converged {
            eprintln!(
                "warning: product spec review did not converge after {} iterations",
                config.review.max_iterations
            );
        }

        if config.interaction == InteractionMode::Milestones {
            state.spec_step = Some(SpecStep::TechnicalSpecGeneration);
            state.save(&state_path)?;
            println!(
                "Product spec complete. Review the spec at docs/product-spec.md, then re-run to continue."
            );
            return Ok(());
        }
    }

    let technical_spec_path = docs_dir.join("technical-spec.md");
    if start_ord <= spec_step_ordinal(&SpecStep::TechnicalSpecGeneration) {
        crate::output::print_step("Generating technical spec from product spec");
        state.spec_step = Some(SpecStep::TechnicalSpecGeneration);
        state.save(&state_path)?;

        let mut cleanup = SingleFileGuard::new(&technical_spec_path);

        let tech_template = loader.load("spec_technical")?;
        let mut tech_ctx = ContextBuilder::new();
        tech_ctx.add_file("product-spec.md", &product_spec_path)?;
        let tech_prompt = template::replace_vars(
            &tech_ctx.apply(&tech_template),
            &[
                ("project_name", &config.project.name),
                ("target_file", "docs/technical-spec.md"),
            ],
        );

        let mut display = StreamDisplay::new();
        display.set_context_stats(tech_ctx.total_tokens(), tech_ctx.block_stats().len());
        let result = invoke_sub_agent(
            &tech_prompt,
            &config.models.spec,
            config.effort.spec,
            Some(GENERATION_TOOLS),
            Some(system_prompt.as_str()),
            Some(project_dir),
            &mut display,
            &config.retry,
            dry_run,
        )
        .await?;

        state.spec_cost_usd += result.cost_usd;
        state.total_cost_usd += result.cost_usd;
        state.save(&state_path)?;

        if !technical_spec_path.exists() {
            bail!("sub agent did not create docs/technical-spec.md");
        }

        cleanup.defuse();
    }

    if start_ord <= spec_step_ordinal(&SpecStep::TechnicalSpecReview { iteration: 0 }) {
        let starting_iteration = match &starting_step {
            SpecStep::TechnicalSpecReview { iteration } if *iteration > 0 => *iteration + 1,
            _ => 1,
        };

        state.spec_step = Some(SpecStep::TechnicalSpecReview {
            iteration: starting_iteration.saturating_sub(1),
        });
        state.save(&state_path)?;

        let tech_review_template = loader.load("spec_review")?;
        let tech_review_prompt = template::replace_vars(
            &tech_review_template,
            &[
                ("project_name", &config.project.name),
                ("target_file", "docs/technical-spec.md"),
            ],
        );

        let technical_spec_path_clone = technical_spec_path.clone();
        let product_spec_path_clone = product_spec_path.clone();
        let tech_review_params = ReviewParams {
            config,
            prompt_template: &tech_review_prompt,
            model: &config.models.review,
            effort: config.effort.review,
            max_iterations: config.review.max_iterations,
            tools: Some(REVIEW_TOOLS),
            system_prompt: Some(system_prompt.as_str()),
            cwd: Some(project_dir),
            dry_run,
        };
        let context_fn = || {
            let tech_path = technical_spec_path_clone.clone();
            let prod_path = product_spec_path_clone.clone();
            async move {
                let mut cb = ContextBuilder::new();
                cb.add_file("technical-spec.md", &tech_path)?;
                cb.add_file("product-spec.md", &prod_path)?;
                Ok(cb)
            }
        };

        let max = config.review.max_iterations;
        let mut display = StreamDisplay::new();
        let mut converged = false;
        for iteration in starting_iteration..=max {
            crate::output::print_step(&format!(
                "Reviewing technical spec, iteration {iteration}/{max}"
            ));
            let iter_result =
                run_review_iteration(&tech_review_params, &context_fn, &mut display).await?;

            state.spec_cost_usd += iter_result.cost_usd;
            state.total_cost_usd += iter_result.cost_usd;
            state.spec_step = Some(SpecStep::TechnicalSpecReview { iteration });
            state.save(&state_path)?;

            if iter_result.verdict == Verdict::Clean {
                converged = true;
                break;
            }
        }

        if !converged {
            eprintln!(
                "warning: technical spec review did not converge after {} iterations",
                config.review.max_iterations
            );
        }

        if config.interaction == InteractionMode::Milestones {
            println!(
                "Technical spec complete. Review the spec at docs/technical-spec.md, then re-run to continue."
            );
        }
    }

    state.spec_status = StageStatus::Complete;
    state.spec_step = None;
    state.save(&state_path)?;

    Ok(())
}
