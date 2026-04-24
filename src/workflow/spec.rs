use anyhow::{Result, bail};

use crate::config::InteractionMode;
use crate::intent::Classification;
use crate::output::StreamDisplay;
use crate::state::{SpecStep, StageStatus, spec_step_ordinal};
use crate::template;
use crate::workflow::IntentContext;
use crate::workflow::cleanup::SingleFileGuard;
use crate::workflow::context::ContextBuilder;
use crate::workflow::review::{ReviewParams, run_review_loop};

const GENERATION_TOOLS: &str = "Read,Write,Edit,Glob,Grep,Bash";
const REVIEW_TOOLS: &str = "Read,Write,Edit,Glob,Grep";

pub async fn run_spec(ctx: &mut IntentContext<'_>, description: &str) -> Result<()> {
    std::fs::create_dir_all(&ctx.specs_dir)?;

    if ctx.intent.classification != Classification::Build {
        return amend_specs(ctx).await;
    }

    let starting_step = ctx
        .intent
        .spec_step
        .clone()
        .unwrap_or(SpecStep::ProductSpecGeneration);
    let start_ord = spec_step_ordinal(&starting_step);

    ctx.intent.spec_status = StageStatus::InProgress;
    ctx.save_intent()?;

    let product_spec_path = ctx.product_spec_path();

    if start_ord <= spec_step_ordinal(&SpecStep::ProductSpecGeneration) {
        crate::output::print_step("Generating product spec from project description");
        ctx.intent.spec_step = Some(SpecStep::ProductSpecGeneration);
        ctx.save_intent()?;

        let mut cleanup = SingleFileGuard::new(&product_spec_path);

        let product_template = ctx.loader.load("spec_product")?;
        let mut cb = ContextBuilder::new();
        cb.add_content("description", description);
        let prompt = template::replace_vars(
            &cb.apply(&product_template),
            &[
                ("project_name", &ctx.config.project.name),
                ("target_file", &product_spec_path.display().to_string()),
            ],
        );

        let mut display = StreamDisplay::new();
        display.set_context_stats(cb.total_tokens(), cb.block_stats().len());
        let result = super::invoke_sub_agent(
            &prompt,
            &ctx.config.spec.model,
            ctx.config.spec.effort,
            Some(GENERATION_TOOLS),
            Some(ctx.system_prompt.as_str()),
            Some(&ctx.work_dir),
            &mut display,
            &ctx.config.retry,
            ctx.dry_run,
        )
        .await?;

        if ctx.dry_run {
            return Ok(());
        }

        ctx.intent.spec_cost_usd += result.cost_usd;
        ctx.intent.total_cost_usd += result.cost_usd;
        ctx.save_intent()?;

        if !product_spec_path.exists() {
            bail!("sub agent did not create {}", product_spec_path.display());
        }

        cleanup.defuse();
    }

    if start_ord <= spec_step_ordinal(&SpecStep::ProductSpecReview { iteration: 0 }) {
        let starting_iteration = match &starting_step {
            SpecStep::ProductSpecReview { iteration } if *iteration > 0 => *iteration + 1,
            _ => 1,
        };

        ctx.intent.spec_step = Some(SpecStep::ProductSpecReview {
            iteration: starting_iteration.saturating_sub(1),
        });
        ctx.save_intent()?;

        let review_template = ctx.loader.load("spec_review")?;
        let review_prompt = template::replace_vars(
            &review_template,
            &[
                ("project_name", &ctx.config.project.name),
                ("target_file", &product_spec_path.display().to_string()),
            ],
        );

        let product_spec_path_clone = product_spec_path.clone();
        let system_prompt_clone = ctx.system_prompt.clone();
        let work_dir_clone = ctx.work_dir.clone();
        let mut review_params = ReviewParams {
            config: ctx.config,
            prompt_template: &review_prompt,
            model: &ctx.config.spec.review_model,
            effort: ctx.config.spec.review_effort,
            max_iterations: ctx.config.spec.max_review_iterations,
            tools: Some(REVIEW_TOOLS),
            system_prompt: Some(&system_prompt_clone),
            cwd: Some(&work_dir_clone),
            dry_run: ctx.dry_run,
            prior_findings: None,
        };
        let context_fn = || {
            let path = product_spec_path_clone.clone();
            async move {
                let mut cb = ContextBuilder::new();
                cb.add_file("product-spec.md", &path)?;
                Ok(cb)
            }
        };

        let converged = run_review_loop(
            &mut review_params,
            ctx.config.spec.review_effort,
            starting_iteration,
            "Reviewing product spec",
            &context_fn,
            |iteration, cost| {
                ctx.intent.spec_cost_usd += cost;
                ctx.intent.total_cost_usd += cost;
                ctx.intent.spec_step = Some(SpecStep::ProductSpecReview { iteration });
                ctx.save_intent()
            },
        )
        .await?;

        if !converged {
            eprintln!(
                "warning: product spec review did not converge after {} iterations",
                ctx.config.spec.max_review_iterations
            );
        }

        if ctx.config.interaction == InteractionMode::Milestones {
            ctx.intent.spec_step = Some(SpecStep::TechnicalSpecGeneration);
            ctx.save_intent()?;
            println!(
                "Product spec complete. Review the spec at {}, then re-run to continue.",
                product_spec_path.display()
            );
            return Ok(());
        }
    }

    let technical_spec_path = ctx.technical_spec_path();
    if start_ord <= spec_step_ordinal(&SpecStep::TechnicalSpecGeneration) {
        crate::output::print_step("Generating technical spec from product spec");
        ctx.intent.spec_step = Some(SpecStep::TechnicalSpecGeneration);
        ctx.save_intent()?;

        let mut cleanup = SingleFileGuard::new(&technical_spec_path);

        let tech_template = ctx.loader.load("spec_technical")?;
        let mut tech_ctx = ContextBuilder::new();
        tech_ctx.add_file("product-spec.md", &product_spec_path)?;
        let tech_prompt = template::replace_vars(
            &tech_ctx.apply(&tech_template),
            &[
                ("project_name", &ctx.config.project.name),
                ("target_file", &technical_spec_path.display().to_string()),
            ],
        );

        let mut display = StreamDisplay::new();
        display.set_context_stats(tech_ctx.total_tokens(), tech_ctx.block_stats().len());
        let result = super::invoke_sub_agent(
            &tech_prompt,
            &ctx.config.spec.model,
            ctx.config.spec.effort,
            Some(GENERATION_TOOLS),
            Some(ctx.system_prompt.as_str()),
            Some(&ctx.work_dir),
            &mut display,
            &ctx.config.retry,
            ctx.dry_run,
        )
        .await?;

        ctx.intent.spec_cost_usd += result.cost_usd;
        ctx.intent.total_cost_usd += result.cost_usd;
        ctx.save_intent()?;

        if !technical_spec_path.exists() {
            bail!("sub agent did not create {}", technical_spec_path.display());
        }

        cleanup.defuse();
    }

    if start_ord <= spec_step_ordinal(&SpecStep::TechnicalSpecReview { iteration: 0 }) {
        let starting_iteration = match &starting_step {
            SpecStep::TechnicalSpecReview { iteration } if *iteration > 0 => *iteration + 1,
            _ => 1,
        };

        ctx.intent.spec_step = Some(SpecStep::TechnicalSpecReview {
            iteration: starting_iteration.saturating_sub(1),
        });
        ctx.save_intent()?;

        let tech_review_template = ctx.loader.load("spec_review")?;
        let tech_review_prompt = template::replace_vars(
            &tech_review_template,
            &[
                ("project_name", &ctx.config.project.name),
                ("target_file", &technical_spec_path.display().to_string()),
            ],
        );

        let technical_spec_path_clone = technical_spec_path.clone();
        let product_spec_path_clone = product_spec_path.clone();
        let system_prompt_clone = ctx.system_prompt.clone();
        let work_dir_clone = ctx.work_dir.clone();
        let mut tech_review_params = ReviewParams {
            config: ctx.config,
            prompt_template: &tech_review_prompt,
            model: &ctx.config.spec.review_model,
            effort: ctx.config.spec.review_effort,
            max_iterations: ctx.config.spec.max_review_iterations,
            tools: Some(REVIEW_TOOLS),
            system_prompt: Some(&system_prompt_clone),
            cwd: Some(&work_dir_clone),
            dry_run: ctx.dry_run,
            prior_findings: None,
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

        let converged = run_review_loop(
            &mut tech_review_params,
            ctx.config.spec.review_effort,
            starting_iteration,
            "Reviewing technical spec",
            &context_fn,
            |iteration, cost| {
                ctx.intent.spec_cost_usd += cost;
                ctx.intent.total_cost_usd += cost;
                ctx.intent.spec_step = Some(SpecStep::TechnicalSpecReview { iteration });
                ctx.save_intent()
            },
        )
        .await?;

        if !converged {
            eprintln!(
                "warning: technical spec review did not converge after {} iterations",
                ctx.config.spec.max_review_iterations
            );
        }

        if ctx.config.interaction == InteractionMode::Milestones {
            println!(
                "Technical spec complete. Review the spec at {}, then re-run to continue.",
                technical_spec_path.display()
            );
        }
    }

    ctx.intent.spec_status = StageStatus::Complete;
    ctx.intent.spec_step = None;
    ctx.save_intent()?;

    Ok(())
}

async fn amend_specs(ctx: &mut IntentContext<'_>) -> Result<()> {
    let product_spec_path = ctx.product_spec_path();
    let technical_spec_path = ctx.technical_spec_path();

    if !product_spec_path.exists() || !technical_spec_path.exists() {
        bail!(
            "cannot amend specs: product.md and technical.md must exist in {}. Run `yoke discover` first.",
            ctx.specs_dir.display()
        );
    }

    crate::output::print_step("Amending specs for intent");

    ctx.intent.spec_status = StageStatus::InProgress;
    ctx.save_intent()?;

    let amend_template = ctx.loader.load("spec_amend")?;

    let mut cb = ContextBuilder::new();
    cb.add_file("product spec", &product_spec_path)?;
    cb.add_file("technical spec", &technical_spec_path)?;
    cb.add_content("intent description", &ctx.intent.description);

    let prompt = template::replace_vars(
        &cb.apply(&amend_template),
        &[
            ("project_name", &ctx.config.project.name),
            ("intent_title", &ctx.intent.title),
            ("intent_id", &ctx.intent.id),
            (
                "product_spec_path",
                &product_spec_path.display().to_string(),
            ),
            (
                "technical_spec_path",
                &technical_spec_path.display().to_string(),
            ),
        ],
    );

    let mut display = StreamDisplay::new();
    display.set_context_stats(cb.total_tokens(), cb.block_stats().len());

    let result = super::invoke_sub_agent(
        &prompt,
        &ctx.config.spec.model,
        ctx.config.spec.effort,
        Some(GENERATION_TOOLS),
        Some(ctx.system_prompt.as_str()),
        Some(&ctx.work_dir),
        &mut display,
        &ctx.config.retry,
        ctx.dry_run,
    )
    .await?;

    ctx.intent.spec_cost_usd += result.cost_usd;
    ctx.intent.total_cost_usd += result.cost_usd;

    let review_template = ctx.loader.load("spec_review")?;
    let review_prompt = template::replace_vars(
        &review_template,
        &[
            ("project_name", &ctx.config.project.name),
            ("target_file", &product_spec_path.display().to_string()),
        ],
    );

    let product_clone = product_spec_path.clone();
    let technical_clone = technical_spec_path.clone();
    let system_prompt = ctx.system_prompt.clone();
    let work_dir_clone = ctx.work_dir.clone();
    let mut review_params = ReviewParams {
        config: ctx.config,
        prompt_template: &review_prompt,
        model: &ctx.config.spec.review_model,
        effort: ctx.config.spec.review_effort,
        max_iterations: ctx.config.spec.max_review_iterations,
        tools: Some(REVIEW_TOOLS),
        system_prompt: Some(&system_prompt),
        cwd: Some(&work_dir_clone),
        dry_run: ctx.dry_run,
        prior_findings: None,
    };
    let context_fn = || {
        let prod = product_clone.clone();
        let tech = technical_clone.clone();
        async move {
            let mut cb = ContextBuilder::new();
            cb.add_file("product spec", &prod)?;
            cb.add_file("technical spec", &tech)?;
            Ok(cb)
        }
    };

    let converged = run_review_loop(
        &mut review_params,
        ctx.config.spec.review_effort,
        1,
        "Reviewing spec amendments",
        &context_fn,
        |_iteration, cost| {
            ctx.intent.spec_cost_usd += cost;
            ctx.intent.total_cost_usd += cost;
            ctx.save_intent()
        },
    )
    .await?;

    if !converged {
        eprintln!(
            "warning: spec amendment review did not converge after {} iterations",
            ctx.config.spec.max_review_iterations
        );
    }

    ctx.intent.spec_status = StageStatus::Complete;
    ctx.intent.spec_step = None;
    ctx.save_intent()?;

    Ok(())
}
