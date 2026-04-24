use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;

use crate::config::InteractionMode;
use crate::intent::Depth;
use crate::output::StreamDisplay;
use crate::state::{PhaseStatus, PhaseStep};
use crate::template;
use crate::workflow::IntentContext;
use crate::workflow::cleanup::FileCleanupGuard;
use crate::workflow::context::{ContextBuilder, estimate_tokens};
use crate::workflow::format_gate_commands;
use crate::workflow::review::{ReviewParams, run_review_loop};

const RESEARCH_TOOLS: &str = "Bash,Read,Write,Edit,Glob,Grep,WebSearch,WebFetch,Agent";
const PLANNING_TOOLS: &str = "Read,Write,Edit,Glob,Grep";
const REVIEW_TOOLS: &str = "Read,Write,Edit,Glob,Grep";
const CODE_REVIEW_TOOLS: &str = "Bash,Read,Write,Edit,Glob,Grep";

pub async fn run_phase(ctx: &mut IntentContext<'_>, phase_number: usize) -> Result<()> {
    let phase_idx = ctx
        .intent
        .phases
        .iter()
        .position(|p| p.number == phase_number)
        .ok_or_else(|| anyhow::anyhow!("phase {phase_number} not found in state"))?;

    let padded = format!("{:03}", phase_number);
    let phase_spec_path = find_phase_spec(&ctx.phases_dir(), phase_number)?;

    let pre_phase_commit = if let Some(ref commit) = ctx.intent.phases[phase_idx].pre_phase_commit {
        Some(commit.clone())
    } else {
        let commit = crate::git::capture_head(&ctx.work_dir).await;
        if commit.is_some() {
            ctx.intent.phases[phase_idx].pre_phase_commit = commit.clone();
            ctx.save_intent()?;
        }
        commit
    };

    let depth = ctx.intent.depth;
    let starting_step = ctx.intent.phases[phase_idx]
        .current_step
        .clone()
        .unwrap_or_else(|| default_starting_step(depth));

    if ctx.intent.phases[phase_idx].status != PhaseStatus::InProgress {
        ctx.intent.phases[phase_idx].status = PhaseStatus::InProgress;
        ctx.intent.phases[phase_idx].started_at = Some(Utc::now());
    }

    let steps = steps_from(&starting_step, depth);

    let result = execute_steps(
        &steps,
        phase_idx,
        phase_number,
        &padded,
        &phase_spec_path,
        &pre_phase_commit,
        ctx,
    )
    .await;

    if let Err(ref e) = result {
        ctx.intent.phases[phase_idx].status = PhaseStatus::Failed;
        let _ = ctx.save_intent();
        let step_label = ctx.intent.phases[phase_idx]
            .current_step
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default();
        eprintln!("phase {phase_number} failed at step {step_label}: {e}");
    }

    result
}

async fn execute_steps(
    steps: &[PhaseStep],
    phase_idx: usize,
    phase_number: usize,
    padded: &str,
    phase_spec_path: &Path,
    pre_phase_commit: &Option<String>,
    ctx: &mut IntentContext<'_>,
) -> Result<()> {
    for step in steps {
        ctx.intent.phases[phase_idx].current_step = Some(step.clone());
        ctx.save_intent()?;

        let phase_title = ctx.intent.phases[phase_idx].title.clone();

        let mut display = StreamDisplay::new();

        match step {
            PhaseStep::Research => {
                crate::output::print_step(&format!(
                    "Researching codebase for phase {phase_number} ({phase_title})"
                ));
                let research_dir = ctx.research_dir();
                std::fs::create_dir_all(&research_dir).context("creating research directory")?;
                let mut cleanup = FileCleanupGuard::new(&research_dir);

                let template_text = ctx.loader.load("research")?;
                let target_file = research_dir.join(format!("phase-{padded}-findings.md"));
                let mut cb = ContextBuilder::new();
                cb.add_file("phase spec", phase_spec_path)?;
                add_spec_context(&mut cb, ctx, padded)?;
                let prompt = template::replace_vars(
                    &cb.apply(&template_text),
                    &[
                        ("project_name", &ctx.config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("target_file", &target_file.display().to_string()),
                    ],
                );
                display.set_context_stats(cb.total_tokens(), cb.block_stats().len());

                let result = super::invoke_sub_agent(
                    &prompt,
                    &ctx.config.phase.research.model,
                    ctx.config.phase.research.effort,
                    Some(RESEARCH_TOOLS),
                    Some(ctx.system_prompt.as_str()),
                    Some(&ctx.work_dir),
                    &mut display,
                    &ctx.config.retry,
                    ctx.dry_run,
                )
                .await?;

                ctx.accumulate_cost(phase_idx, result.cost_usd, "research");
                ctx.save_intent()?;

                let research_files = glob_research_files(&ctx.research_dir(), phase_number);
                if research_files.is_empty() {
                    eprintln!(
                        "warning: no research files created for phase {phase_number} (this may be expected)"
                    );
                }

                cleanup.defuse();
            }

            PhaseStep::SpecExtract => {
                let product_spec = ctx.product_spec_path();
                let technical_spec = ctx.technical_spec_path();

                if !product_spec.exists() && !technical_spec.exists() {
                    // No specs to extract from; subsequent steps fall back to full injection.
                } else if specs_below_threshold(&product_spec, &technical_spec, ctx) {
                    crate::output::print_step(&format!(
                        "Skipping spec extraction for phase {phase_number} (specs below token threshold)"
                    ));
                } else {
                    crate::output::print_step(&format!(
                        "Extracting relevant specs for phase {phase_number} ({phase_title})"
                    ));
                    let extracts_dir = ctx.extracts_dir();
                    std::fs::create_dir_all(&extracts_dir)
                        .context("creating extracts directory")?;

                    let template_text = ctx.loader.load("spec_extract")?;
                    let target_file = extracts_dir.join(format!("phase-{padded}-specs.md"));
                    let mut cb = ContextBuilder::new();
                    cb.add_file("phase spec", phase_spec_path)?;
                    cb.add_file("product spec", &product_spec)?;
                    if technical_spec.exists() {
                        cb.add_file("technical spec", &technical_spec)?;
                    }

                    let prompt = template::replace_vars(
                        &cb.apply(&template_text),
                        &[
                            ("project_name", &ctx.config.project.name),
                            ("phase_number", &phase_number.to_string()),
                            ("phase_number_padded", padded),
                            ("target_file", &target_file.display().to_string()),
                        ],
                    );
                    display.set_context_stats(cb.total_tokens(), cb.block_stats().len());

                    let result = super::invoke_sub_agent(
                        &prompt,
                        &ctx.config.phase.spec_extract.model,
                        ctx.config.phase.spec_extract.effort,
                        Some(PLANNING_TOOLS),
                        Some(ctx.system_prompt.as_str()),
                        Some(&ctx.work_dir),
                        &mut display,
                        &ctx.config.retry,
                        ctx.dry_run,
                    )
                    .await?;

                    ctx.accumulate_cost(phase_idx, result.cost_usd, "spec_extract");
                    ctx.save_intent()?;

                    if !ctx.dry_run && !target_file.exists() {
                        eprintln!(
                            "warning: spec extraction did not produce {}; subsequent steps will use full specs",
                            target_file.display()
                        );
                    }
                }
            }

            PhaseStep::Planning => {
                crate::output::print_step(&format!(
                    "Building implementation plan for phase {phase_number} ({phase_title})"
                ));
                let plans_dir = ctx.plans_dir();
                std::fs::create_dir_all(&plans_dir).context("creating plans directory")?;
                let mut cleanup = FileCleanupGuard::new(&plans_dir);

                let template_text = ctx.loader.load("phase_plan_generate")?;
                let target_file = plans_dir.join(format!("phase-{padded}.md"));
                let mut cb = ContextBuilder::new();
                add_reference_context(&mut cb, phase_spec_path, ctx, padded, phase_number)?;
                if let Some(handoff_path) =
                    find_most_recent_handoff(&ctx.handoffs_dir(), phase_number)
                {
                    cb.add_file("most recent handoff", &handoff_path)?;
                }
                let prompt = template::replace_vars(
                    &cb.apply(&template_text),
                    &[
                        ("project_name", &ctx.config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("target_file", &target_file.display().to_string()),
                    ],
                );
                display.set_context_stats(cb.total_tokens(), cb.block_stats().len());

                let result = super::invoke_sub_agent(
                    &prompt,
                    &ctx.config.phase.planning.model,
                    ctx.config.phase.planning.effort,
                    Some(PLANNING_TOOLS),
                    Some(ctx.system_prompt.as_str()),
                    Some(&ctx.work_dir),
                    &mut display,
                    &ctx.config.retry,
                    ctx.dry_run,
                )
                .await?;

                ctx.accumulate_cost(phase_idx, result.cost_usd, "planning");
                ctx.save_intent()?;

                if !ctx.dry_run && !target_file.exists() {
                    bail!("sub agent did not create {}", target_file.display());
                }

                cleanup.defuse();
            }

            PhaseStep::PlanReview {
                iteration: saved_iter,
            } => {
                let starting_iteration = if *saved_iter > 0 { *saved_iter + 1 } else { 1 };

                let review_template = ctx.loader.load("plan_review")?;
                let plans_dir = ctx.plans_dir();
                let plan_file = plans_dir.join(format!("phase-{padded}.md"));
                let review_prompt = template::replace_vars(
                    &review_template,
                    &[
                        ("project_name", &ctx.config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("target_file", &plan_file.display().to_string()),
                    ],
                );

                let plan_file_clone = plan_file.clone();
                let phase_spec_clone = phase_spec_path.to_path_buf();
                let specs_dir_clone = ctx.specs_dir.clone();
                let extracts_dir_clone = ctx.extracts_dir();
                let research_dir_clone = ctx.research_dir();
                let padded_clone = padded.to_string();
                let system_prompt_clone = ctx.system_prompt.clone();
                let work_dir_clone = ctx.work_dir.clone();
                let mut review_params = ReviewParams {
                    config: ctx.config,
                    prompt_template: &review_prompt,
                    model: &ctx.config.phase.plan_review.model,
                    effort: ctx.config.phase.plan_review.effort,
                    max_iterations: ctx.config.phase.plan_review.max_iterations,
                    tools: Some(REVIEW_TOOLS),
                    system_prompt: Some(&system_prompt_clone),
                    cwd: Some(&work_dir_clone),
                    dry_run: ctx.dry_run,
                    prior_findings: None,
                };
                let context_fn = || {
                    let path = plan_file_clone.clone();
                    let spec = phase_spec_clone.clone();
                    let specs = specs_dir_clone.clone();
                    let extracts = extracts_dir_clone.clone();
                    let research = research_dir_clone.clone();
                    let pad = padded_clone.clone();
                    async move {
                        let mut cb = ContextBuilder::new();
                        cb.add_file("phase plan", &path)?;
                        cb.add_file("phase spec", &spec)?;
                        add_spec_context_static(&mut cb, &specs, &extracts, &pad)?;
                        for research_path in glob_research_files(&research, phase_number) {
                            let label = research_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "research".to_string());
                            cb.add_file(&label, &research_path)?;
                        }
                        Ok(cb)
                    }
                };

                let converged = run_review_loop(
                    &mut review_params,
                    ctx.config.phase.plan_review.effort,
                    starting_iteration,
                    &format!("Reviewing plan for phase {phase_number} ({phase_title})"),
                    &context_fn,
                    |iteration, cost| {
                        ctx.accumulate_cost(
                            phase_idx,
                            cost,
                            &format!("plan review (iteration {iteration})"),
                        );
                        ctx.intent.phases[phase_idx].current_step =
                            Some(PhaseStep::PlanReview { iteration });
                        ctx.save_intent()
                    },
                )
                .await?;

                if !converged {
                    eprintln!(
                        "warning: plan review did not converge after {} iterations",
                        ctx.config.phase.plan_review.max_iterations
                    );
                    if ctx.config.interaction == InteractionMode::Milestones {
                        println!(
                            "Plan review did not converge for phase {phase_number}. Review the output above and re-run when ready."
                        );
                        return Ok(());
                    }
                }
            }

            PhaseStep::Execution => {
                crate::output::print_step(&format!(
                    "Executing phase {phase_number} ({phase_title})"
                ));
                let template_text = ctx.loader.load("execution")?;
                let plans_dir = ctx.plans_dir();
                let plan_file = plans_dir.join(format!("phase-{padded}.md"));
                let mut cb = ContextBuilder::new();
                if plan_file.exists() {
                    cb.add_file("phase plan", &plan_file)?;
                } else {
                    cb.add_content("phase plan", &ctx.intent.description);
                }
                add_reference_context(&mut cb, phase_spec_path, ctx, padded, phase_number)?;
                if let Some(handoff_path) =
                    find_most_recent_handoff(&ctx.handoffs_dir(), phase_number)
                {
                    cb.add_file("most recent handoff", &handoff_path)?;
                }
                let gate_commands_text = format_gate_commands(&ctx.config.gate_commands);
                let prompt = template::replace_vars(
                    &cb.apply(&template_text),
                    &[
                        ("project_name", &ctx.config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("gate_commands", &gate_commands_text),
                    ],
                );
                display.set_context_stats(cb.total_tokens(), cb.block_stats().len());

                let result = super::invoke_sub_agent(
                    &prompt,
                    &ctx.config.phase.execution.model,
                    ctx.config.phase.execution.effort,
                    None,
                    Some(ctx.system_prompt.as_str()),
                    Some(&ctx.work_dir),
                    &mut display,
                    &ctx.config.retry,
                    ctx.dry_run,
                )
                .await?;

                ctx.accumulate_cost(phase_idx, result.cost_usd, "execution");
                ctx.save_intent()?;

                if !result.result_text.is_empty() {
                    let summaries_dir = ctx.intent_dir.join("summaries");
                    std::fs::create_dir_all(&summaries_dir)
                        .context("creating summaries directory")?;
                    let summary_path = summaries_dir.join(format!("phase-{padded}-execution.md"));
                    std::fs::write(&summary_path, &result.result_text).with_context(|| {
                        format!("writing execution summary to {}", summary_path.display())
                    })?;
                }
            }

            PhaseStep::CodeReview {
                iteration: saved_iter,
            } => {
                let starting_iteration = if *saved_iter > 0 { *saved_iter + 1 } else { 1 };

                let code_review_template = ctx.loader.load("code_review")?;
                let plans_dir = ctx.plans_dir();
                let plan_file = plans_dir.join(format!("phase-{padded}.md"));
                let gate_commands_text = format_gate_commands(&ctx.config.gate_commands);
                let code_review_prompt = template::replace_vars(
                    &code_review_template,
                    &[
                        ("project_name", &ctx.config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("gate_commands", &gate_commands_text),
                    ],
                );

                let plan_file_clone = plan_file.clone();
                let phase_spec_clone = phase_spec_path.to_path_buf();
                let specs_dir_clone = ctx.specs_dir.clone();
                let extracts_dir_clone = ctx.extracts_dir();
                let research_dir_clone = ctx.research_dir();
                let padded_clone = padded.to_string();
                let system_prompt_clone = ctx.system_prompt.clone();
                let work_dir_clone = ctx.work_dir.clone();
                let mut review_params = ReviewParams {
                    config: ctx.config,
                    prompt_template: &code_review_prompt,
                    model: &ctx.config.phase.code_review.model,
                    effort: ctx.config.phase.code_review.effort,
                    max_iterations: ctx.config.phase.code_review.max_iterations,
                    tools: Some(CODE_REVIEW_TOOLS),
                    system_prompt: Some(&system_prompt_clone),
                    cwd: Some(&work_dir_clone),
                    dry_run: ctx.dry_run,
                    prior_findings: None,
                };
                let context_fn = || {
                    let plan = plan_file_clone.clone();
                    let spec = phase_spec_clone.clone();
                    let specs = specs_dir_clone.clone();
                    let extracts = extracts_dir_clone.clone();
                    let research = research_dir_clone.clone();
                    let pad = padded_clone.clone();
                    async move {
                        let mut cb = ContextBuilder::new();
                        cb.add_file("phase plan", &plan)?;
                        cb.add_file("phase spec", &spec)?;
                        add_spec_context_static(&mut cb, &specs, &extracts, &pad)?;
                        for research_path in glob_research_files(&research, phase_number) {
                            let label = research_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "research".to_string());
                            cb.add_file(&label, &research_path)?;
                        }
                        Ok(cb)
                    }
                };

                let converged = run_review_loop(
                    &mut review_params,
                    ctx.config.phase.code_review.effort,
                    starting_iteration,
                    &format!("Reviewing code for phase {phase_number} ({phase_title})"),
                    &context_fn,
                    |iteration, cost| {
                        ctx.accumulate_cost(
                            phase_idx,
                            cost,
                            &format!("code review (iteration {iteration})"),
                        );
                        ctx.intent.phases[phase_idx].current_step =
                            Some(PhaseStep::CodeReview { iteration });
                        ctx.save_intent()
                    },
                )
                .await?;

                if !converged {
                    eprintln!(
                        "warning: code review did not converge after {} iterations",
                        ctx.config.phase.code_review.max_iterations
                    );
                    if ctx.config.interaction == InteractionMode::Milestones {
                        println!(
                            "Code review did not converge for phase {phase_number}. Review the output above and re-run when ready."
                        );
                        return Ok(());
                    }
                }
            }

            PhaseStep::Handoff => {
                crate::output::print_step(&format!(
                    "Writing handoff notes for phase {phase_number} ({phase_title})"
                ));
                let handoffs_dir = ctx.handoffs_dir();
                std::fs::create_dir_all(&handoffs_dir).context("creating handoffs directory")?;
                let mut cleanup = FileCleanupGuard::new(&handoffs_dir);

                let template_text = ctx.loader.load("handoff")?;
                let target_file = handoffs_dir.join(format!("phase-{padded}.md"));
                let plans_dir = ctx.plans_dir();
                let plan_file = plans_dir.join(format!("phase-{padded}.md"));
                let execution_summary_path = ctx
                    .intent_dir
                    .join(format!("summaries/phase-{padded}-execution.md"));
                let mut cb = ContextBuilder::new();
                cb.add_file("phase plan", &plan_file)?;
                if execution_summary_path.exists() {
                    cb.add_file("execution summary", &execution_summary_path)?;
                }

                let knowledge_content = crate::knowledge::load(ctx.project_dir)?;
                if !knowledge_content.is_empty() {
                    cb.add_content("knowledge base", &knowledge_content);
                }

                if let Some(commit) = pre_phase_commit {
                    let diff_stat_text = crate::git::diff_stat(&ctx.work_dir, commit).await;
                    if !diff_stat_text.is_empty() {
                        cb.add_content("git diff summary", &diff_stat_text);
                    }
                }

                let prompt = template::replace_vars(
                    &cb.apply(&template_text),
                    &[
                        ("project_name", &ctx.config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("target_file", &target_file.display().to_string()),
                    ],
                );
                display.set_context_stats(cb.total_tokens(), cb.block_stats().len());

                let result = super::invoke_sub_agent(
                    &prompt,
                    &ctx.config.phase.handoff.model,
                    ctx.config.phase.handoff.effort,
                    Some(REVIEW_TOOLS),
                    Some(ctx.system_prompt.as_str()),
                    Some(&ctx.work_dir),
                    &mut display,
                    &ctx.config.retry,
                    ctx.dry_run,
                )
                .await?;

                ctx.accumulate_cost(phase_idx, result.cost_usd, "handoff");
                ctx.save_intent()?;

                if !ctx.dry_run && !target_file.exists() {
                    bail!("sub agent did not create {}", target_file.display());
                }

                if !ctx.dry_run && execution_summary_path.exists() {
                    let _ = std::fs::remove_file(&execution_summary_path);
                }

                let extract_path = ctx.extracts_dir().join(format!("phase-{padded}-specs.md"));
                if !ctx.dry_run && extract_path.exists() {
                    let _ = std::fs::remove_file(&extract_path);
                }

                cleanup.defuse();
            }

            PhaseStep::Commit => {
                crate::output::print_step(&format!(
                    "Committing changes for phase {phase_number} ({phase_title})"
                ));
                if ctx.config.git.auto_commit && pre_phase_commit.is_some() {
                    let title = &ctx.intent.phases[phase_idx].title;
                    let message = format!("yoke: complete phase {padded} - {title}");
                    if let Err(e) = crate::git::commit_all(&ctx.work_dir, &message).await {
                        eprintln!("warning: git commit failed: {e}");
                    }
                }
            }
        }
    }

    ctx.intent.phases[phase_idx].status = PhaseStatus::Completed;
    ctx.intent.phases[phase_idx].current_step = None;
    ctx.intent.phases[phase_idx].completed_at = Some(Utc::now());
    ctx.save_intent()?;

    Ok(())
}

pub fn default_starting_step(depth: Depth) -> PhaseStep {
    match depth {
        Depth::Full | Depth::Light => PhaseStep::SpecExtract,
        Depth::Minimal => PhaseStep::Research,
    }
}

fn steps_from(starting: &PhaseStep, depth: Depth) -> Vec<PhaseStep> {
    let all: Vec<PhaseStep> = match depth {
        Depth::Full | Depth::Light => vec![
            PhaseStep::SpecExtract,
            PhaseStep::Research,
            PhaseStep::Planning,
            PhaseStep::PlanReview { iteration: 0 },
            PhaseStep::Execution,
            PhaseStep::CodeReview { iteration: 0 },
            PhaseStep::Handoff,
            PhaseStep::Commit,
        ],
        Depth::Minimal => vec![
            PhaseStep::Research,
            PhaseStep::Execution,
            PhaseStep::CodeReview { iteration: 0 },
            PhaseStep::Commit,
        ],
    };

    let start_idx = all
        .iter()
        .position(|s| step_ordinal(s) == step_ordinal(starting))
        .unwrap_or(0);

    let mut result = all[start_idx..].to_vec();
    result[0] = starting.clone();
    result
}

fn step_ordinal(step: &PhaseStep) -> u8 {
    match step {
        PhaseStep::SpecExtract => 0,
        PhaseStep::Research => 1,
        PhaseStep::Planning => 2,
        PhaseStep::PlanReview { .. } => 3,
        PhaseStep::Execution => 4,
        PhaseStep::CodeReview { .. } => 5,
        PhaseStep::Handoff => 6,
        PhaseStep::Commit => 7,
    }
}

fn specs_below_threshold(
    product_spec: &Path,
    technical_spec: &Path,
    ctx: &IntentContext<'_>,
) -> bool {
    let threshold = ctx.config.phase.spec_extract.threshold;
    if threshold == 0 {
        return false;
    }
    let mut total = 0;
    if product_spec.exists()
        && let Ok(content) = std::fs::read_to_string(product_spec)
    {
        total += estimate_tokens(&content);
    }
    if technical_spec.exists()
        && let Ok(content) = std::fs::read_to_string(technical_spec)
    {
        total += estimate_tokens(&content);
    }
    total < threshold
}

fn add_reference_context(
    cb: &mut ContextBuilder,
    phase_spec_path: &Path,
    ctx: &IntentContext<'_>,
    padded: &str,
    phase_number: usize,
) -> Result<()> {
    cb.add_file("phase spec", phase_spec_path)?;
    add_spec_context(cb, ctx, padded)?;
    for research_path in glob_research_files(&ctx.research_dir(), phase_number) {
        let label = research_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "research".to_string());
        cb.add_file(&label, &research_path)?;
    }
    Ok(())
}

fn add_spec_context(cb: &mut ContextBuilder, ctx: &IntentContext<'_>, padded: &str) -> Result<()> {
    add_spec_context_static(cb, &ctx.specs_dir, &ctx.extracts_dir(), padded)
}

fn add_spec_context_static(
    cb: &mut ContextBuilder,
    specs_dir: &Path,
    extracts_dir: &Path,
    padded: &str,
) -> Result<()> {
    let extract_path = extracts_dir.join(format!("phase-{padded}-specs.md"));
    if extract_path.exists() {
        cb.add_file("relevant specifications", &extract_path)?;
        return Ok(());
    }
    add_specs_to_context(cb, specs_dir)
}

fn add_specs_to_context(cb: &mut ContextBuilder, specs_dir: &Path) -> Result<()> {
    let product_spec = specs_dir.join("product.md");
    if product_spec.exists() {
        cb.add_file("product spec", &product_spec)?;
    }
    let technical_spec = specs_dir.join("technical.md");
    if technical_spec.exists() {
        cb.add_file("technical spec", &technical_spec)?;
    }
    Ok(())
}

fn find_phase_spec(phases_dir: &Path, phase_number: usize) -> Result<PathBuf> {
    if !phases_dir.exists() {
        bail!(
            "phases directory not found at {}. Run planning first to generate phase specs.",
            phases_dir.display()
        );
    }

    let entries = std::fs::read_dir(phases_dir)
        .with_context(|| format!("reading {}", phases_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Some((num, _)) = crate::workflow::plan::parse_phase_filename(&name_str)
            && num == phase_number
        {
            return Ok(entry.path());
        }
    }

    bail!(
        "no phase spec file found for phase {phase_number} in {}",
        phases_dir.display()
    )
}

pub fn find_most_recent_handoff(handoffs_dir: &Path, before_phase: usize) -> Option<PathBuf> {
    if !handoffs_dir.exists() {
        return None;
    }

    let entries = std::fs::read_dir(handoffs_dir).ok()?;
    let mut best: Option<(usize, PathBuf)> = None;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Some(num) = parse_handoff_number(&name_str)
            && num < before_phase
            && best.as_ref().is_none_or(|(b, _)| num > *b)
        {
            best = Some((num, entry.path()));
        }
    }

    best.map(|(_, path)| path)
}

fn parse_handoff_number(filename: &str) -> Option<usize> {
    let stem = filename.strip_suffix(".md")?;
    let num_str = stem.strip_prefix("phase-")?;
    num_str.parse().ok()
}

pub fn glob_research_files(research_dir: &Path, phase_number: usize) -> Vec<PathBuf> {
    if !research_dir.exists() {
        return Vec::new();
    }

    let prefix = format!("phase-{:03}-", phase_number);
    let entries = match std::fs::read_dir(research_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut results: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with(&prefix) && name_str.ends_with(".md")
        })
        .map(|entry| entry.path())
        .collect();

    results.sort();
    results
}

pub fn parse_step_name(name: &str) -> Result<PhaseStep> {
    match name.to_lowercase().as_str() {
        "research" => Ok(PhaseStep::Research),
        "spec_extract" | "spec-extract" => Ok(PhaseStep::SpecExtract),
        "planning" => Ok(PhaseStep::Planning),
        "plan_review" | "plan-review" => Ok(PhaseStep::PlanReview { iteration: 0 }),
        "execution" => Ok(PhaseStep::Execution),
        "code_review" | "code-review" => Ok(PhaseStep::CodeReview { iteration: 0 }),
        "handoff" => Ok(PhaseStep::Handoff),
        "commit" => Ok(PhaseStep::Commit),
        _ => bail!(
            "unknown step name: {name}. Valid steps: research, spec_extract, planning, plan_review, execution, code_review, handoff, commit"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_step_name_all_variants() {
        assert_eq!(parse_step_name("research").unwrap(), PhaseStep::Research);
        assert_eq!(
            parse_step_name("spec_extract").unwrap(),
            PhaseStep::SpecExtract
        );
        assert_eq!(
            parse_step_name("spec-extract").unwrap(),
            PhaseStep::SpecExtract
        );
        assert_eq!(parse_step_name("planning").unwrap(), PhaseStep::Planning);
        assert_eq!(
            parse_step_name("plan_review").unwrap(),
            PhaseStep::PlanReview { iteration: 0 }
        );
        assert_eq!(
            parse_step_name("plan-review").unwrap(),
            PhaseStep::PlanReview { iteration: 0 }
        );
        assert_eq!(parse_step_name("execution").unwrap(), PhaseStep::Execution);
        assert_eq!(
            parse_step_name("code_review").unwrap(),
            PhaseStep::CodeReview { iteration: 0 }
        );
        assert_eq!(
            parse_step_name("code-review").unwrap(),
            PhaseStep::CodeReview { iteration: 0 }
        );
        assert_eq!(parse_step_name("handoff").unwrap(), PhaseStep::Handoff);
        assert_eq!(parse_step_name("commit").unwrap(), PhaseStep::Commit);
    }

    #[test]
    fn parse_step_name_case_insensitive() {
        assert_eq!(parse_step_name("Research").unwrap(), PhaseStep::Research);
        assert_eq!(parse_step_name("EXECUTION").unwrap(), PhaseStep::Execution);
        assert_eq!(
            parse_step_name("Code_Review").unwrap(),
            PhaseStep::CodeReview { iteration: 0 }
        );
    }

    #[test]
    fn parse_step_name_invalid() {
        assert!(parse_step_name("unknown").is_err());
        assert!(parse_step_name("").is_err());
        assert!(parse_step_name("researchx").is_err());
    }

    #[test]
    fn find_most_recent_handoff_none_when_no_dir() {
        let result = find_most_recent_handoff(Path::new("/tmp/yoke_nonexistent_dir_99999"), 5);
        assert!(result.is_none());
    }

    #[test]
    fn find_most_recent_handoff_picks_highest_below_threshold() {
        let dir = std::env::temp_dir().join("yoke_handoff_test_ctx");
        let _ = std::fs::create_dir_all(&dir);

        std::fs::write(dir.join("phase-001.md"), "h1").unwrap();
        std::fs::write(dir.join("phase-002.md"), "h2").unwrap();
        std::fs::write(dir.join("phase-004.md"), "h4").unwrap();

        let result = find_most_recent_handoff(&dir, 3);
        assert_eq!(
            result
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string()),
            Some("phase-002.md".to_string())
        );

        let result = find_most_recent_handoff(&dir, 5);
        assert_eq!(
            result
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string()),
            Some("phase-004.md".to_string())
        );

        let result = find_most_recent_handoff(&dir, 1);
        assert!(result.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn glob_research_files_empty_when_no_dir() {
        let result = glob_research_files(Path::new("/tmp/yoke_nonexistent_dir_99998"), 1);
        assert!(result.is_empty());
    }

    #[test]
    fn glob_research_files_filters_by_phase() {
        let dir = std::env::temp_dir().join("yoke_research_glob_test_ctx");
        let _ = std::fs::create_dir_all(&dir);

        std::fs::write(dir.join("phase-001-findings.md"), "f1").unwrap();
        std::fs::write(dir.join("phase-001-api-notes.md"), "f2").unwrap();
        std::fs::write(dir.join("phase-002-findings.md"), "f3").unwrap();
        std::fs::write(dir.join("unrelated.md"), "x").unwrap();

        let result = glob_research_files(&dir, 1);
        assert_eq!(result.len(), 2);
        for path in &result {
            let name = path.file_name().unwrap().to_string_lossy();
            assert!(name.starts_with("phase-001-"));
        }

        let result = glob_research_files(&dir, 2);
        assert_eq!(result.len(), 1);

        let result = glob_research_files(&dir, 3);
        assert!(result.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn steps_from_spec_extract_returns_all() {
        let steps = steps_from(&PhaseStep::SpecExtract, Depth::Full);
        assert_eq!(steps.len(), 8);
        assert_eq!(step_ordinal(&steps[0]), 0);
        assert_eq!(step_ordinal(&steps[7]), 7);
    }

    #[test]
    fn steps_from_research_skips_spec_extract() {
        let steps = steps_from(&PhaseStep::Research, Depth::Full);
        assert_eq!(steps.len(), 7);
        assert_eq!(step_ordinal(&steps[0]), 1);
    }

    #[test]
    fn steps_from_execution_returns_tail() {
        let steps = steps_from(&PhaseStep::Execution, Depth::Full);
        assert_eq!(steps.len(), 4);
        assert_eq!(step_ordinal(&steps[0]), 4);
    }

    #[test]
    fn steps_from_commit_returns_one() {
        let steps = steps_from(&PhaseStep::Commit, Depth::Full);
        assert_eq!(steps.len(), 1);
        assert_eq!(step_ordinal(&steps[0]), 7);
    }

    #[test]
    fn steps_from_plan_review_preserves_iteration() {
        let steps = steps_from(&PhaseStep::PlanReview { iteration: 3 }, Depth::Full);
        assert_eq!(steps[0], PhaseStep::PlanReview { iteration: 3 });
    }

    #[test]
    fn steps_from_code_review_preserves_iteration() {
        let steps = steps_from(&PhaseStep::CodeReview { iteration: 2 }, Depth::Full);
        assert_eq!(steps[0], PhaseStep::CodeReview { iteration: 2 });
    }

    #[test]
    fn default_starting_step_full() {
        assert_eq!(default_starting_step(Depth::Full), PhaseStep::SpecExtract);
    }

    #[test]
    fn default_starting_step_light() {
        assert_eq!(default_starting_step(Depth::Light), PhaseStep::SpecExtract);
    }

    #[test]
    fn default_starting_step_minimal() {
        assert_eq!(default_starting_step(Depth::Minimal), PhaseStep::Research);
    }

    #[test]
    fn steps_from_full_depth_returns_8() {
        let steps = steps_from(&PhaseStep::SpecExtract, Depth::Full);
        assert_eq!(steps.len(), 8);
        assert_eq!(steps[0], PhaseStep::SpecExtract);
        assert_eq!(steps[1], PhaseStep::Research);
        assert_eq!(steps[2], PhaseStep::Planning);
        assert_eq!(steps[3], PhaseStep::PlanReview { iteration: 0 });
        assert_eq!(steps[4], PhaseStep::Execution);
        assert_eq!(steps[5], PhaseStep::CodeReview { iteration: 0 });
        assert_eq!(steps[6], PhaseStep::Handoff);
        assert_eq!(steps[7], PhaseStep::Commit);
    }

    #[test]
    fn steps_from_light_depth_returns_8() {
        let steps = steps_from(&PhaseStep::SpecExtract, Depth::Light);
        assert_eq!(steps.len(), 8);
        assert_eq!(steps[0], PhaseStep::SpecExtract);
        assert_eq!(steps[7], PhaseStep::Commit);
    }

    #[test]
    fn steps_from_minimal_depth_returns_4() {
        let steps = steps_from(&PhaseStep::Research, Depth::Minimal);
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0], PhaseStep::Research);
        assert_eq!(steps[1], PhaseStep::Execution);
        assert_eq!(steps[2], PhaseStep::CodeReview { iteration: 0 });
        assert_eq!(steps[3], PhaseStep::Commit);
    }

    #[test]
    fn steps_from_minimal_starting_at_execution() {
        let steps = steps_from(&PhaseStep::Execution, Depth::Minimal);
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], PhaseStep::Execution);
        assert_eq!(steps[1], PhaseStep::CodeReview { iteration: 0 });
        assert_eq!(steps[2], PhaseStep::Commit);
    }

    #[test]
    fn steps_from_minimal_code_review_preserves_iteration() {
        let steps = steps_from(&PhaseStep::CodeReview { iteration: 2 }, Depth::Minimal);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0], PhaseStep::CodeReview { iteration: 2 });
        assert_eq!(steps[1], PhaseStep::Commit);
    }

    #[test]
    fn parse_handoff_number_valid() {
        assert_eq!(parse_handoff_number("phase-001.md"), Some(1));
        assert_eq!(parse_handoff_number("phase-042.md"), Some(42));
        assert_eq!(parse_handoff_number("phase-1.md"), Some(1));
    }

    #[test]
    fn parse_handoff_number_invalid() {
        assert_eq!(parse_handoff_number("not-a-phase.md"), None);
        assert_eq!(parse_handoff_number("phase-abc.md"), None);
        assert_eq!(parse_handoff_number("phase-001.txt"), None);
    }
}
