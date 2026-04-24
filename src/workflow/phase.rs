use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;

use crate::config::{InteractionMode, YokeConfig};
use crate::output::StreamDisplay;
use crate::state::{PhaseStatus, PhaseStep, StepCost, YokeState};
use crate::template;
use crate::workflow::cleanup::FileCleanupGuard;
use crate::workflow::context::{ContextBuilder, estimate_tokens};
use crate::workflow::review::{ReviewParams, run_review_loop};
use crate::workflow::{build_system_prompt, format_gate_commands, invoke_sub_agent, prompt_loader};

const RESEARCH_TOOLS: &str = "Bash,Read,Write,Edit,Glob,Grep,WebSearch,WebFetch,Agent";
const PLANNING_TOOLS: &str = "Read,Write,Edit,Glob,Grep";
const REVIEW_TOOLS: &str = "Read,Write,Edit,Glob,Grep";
const CODE_REVIEW_TOOLS: &str = "Bash,Read,Write,Edit,Glob,Grep";

pub async fn run_phase(
    project_dir: &Path,
    phase_number: usize,
    config: &YokeConfig,
    state: &mut YokeState,
    dry_run: bool,
) -> Result<()> {
    let phase_idx = state
        .phases
        .iter()
        .position(|p| p.number == phase_number)
        .ok_or_else(|| anyhow::anyhow!("phase {phase_number} not found in state"))?;

    let padded = format!("{:03}", phase_number);
    let docs_dir = project_dir.join("docs");
    let phase_spec_path = find_phase_spec(project_dir, phase_number)?;

    let loader = prompt_loader(project_dir);
    let system_prompt = build_system_prompt(config, &loader)?;
    let state_path = project_dir.join(".yoke/state.json");

    let pre_phase_commit = if let Some(ref commit) = state.phases[phase_idx].pre_phase_commit {
        Some(commit.clone())
    } else {
        let commit = capture_git_head(project_dir).await;
        if commit.is_some() {
            state.phases[phase_idx].pre_phase_commit = commit.clone();
            state.save(&state_path)?;
        }
        commit
    };

    let starting_step = state.phases[phase_idx]
        .current_step
        .clone()
        .unwrap_or(PhaseStep::SpecExtract);

    if state.phases[phase_idx].status != PhaseStatus::InProgress {
        state.phases[phase_idx].status = PhaseStatus::InProgress;
        state.phases[phase_idx].started_at = Some(Utc::now());
    }

    let steps = steps_from(&starting_step);

    let result = execute_steps(
        &steps,
        phase_idx,
        phase_number,
        &padded,
        project_dir,
        &docs_dir,
        &phase_spec_path,
        &pre_phase_commit,
        &loader,
        Some(system_prompt.as_str()),
        config,
        state,
        &state_path,
        dry_run,
    )
    .await;

    if let Err(ref e) = result {
        state.phases[phase_idx].status = PhaseStatus::Failed;
        let _ = state.save(&state_path);
        let step_label = state.phases[phase_idx]
            .current_step
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default();
        eprintln!("phase {phase_number} failed at step {step_label}: {e}");
    }

    result
}

#[allow(clippy::too_many_arguments)]
async fn execute_steps(
    steps: &[PhaseStep],
    phase_idx: usize,
    phase_number: usize,
    padded: &str,
    project_dir: &Path,
    docs_dir: &Path,
    phase_spec_path: &Path,
    pre_phase_commit: &Option<String>,
    loader: &crate::prompts::PromptLoader,
    system_prompt_ref: Option<&str>,
    config: &YokeConfig,
    state: &mut YokeState,
    state_path: &Path,
    dry_run: bool,
) -> Result<()> {
    for step in steps {
        state.phases[phase_idx].current_step = Some(step.clone());
        state.save(state_path)?;

        let phase_title = state.phases[phase_idx].title.clone();

        let mut display = StreamDisplay::new();

        match step {
            PhaseStep::Research => {
                crate::output::print_step(&format!(
                    "Researching codebase for phase {phase_number} ({phase_title})"
                ));
                let research_dir = docs_dir.join("research");
                std::fs::create_dir_all(&research_dir)
                    .context("creating docs/research directory")?;
                let mut cleanup = FileCleanupGuard::new(&research_dir);

                let template_text = loader.load("research")?;
                let target_file = format!("docs/research/phase-{padded}-findings.md");
                let mut ctx = ContextBuilder::new();
                ctx.add_file("phase spec", phase_spec_path)?;
                add_spec_context(&mut ctx, docs_dir, padded)?;
                let prompt = template::replace_vars(
                    &ctx.apply(&template_text),
                    &[
                        ("project_name", &config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("target_file", &target_file),
                    ],
                );
                display.set_context_stats(ctx.total_tokens(), ctx.block_stats().len());

                let result = invoke_sub_agent(
                    &prompt,
                    &config.models.research,
                    config.effort.research,
                    Some(RESEARCH_TOOLS),
                    system_prompt_ref,
                    Some(project_dir),
                    &mut display,
                    &config.retry,
                    dry_run,
                )
                .await?;

                accumulate_cost(state, phase_idx, result.cost_usd, "research");
                state.save(state_path)?;

                let research_files = glob_research_files(project_dir, phase_number);
                if research_files.is_empty() {
                    eprintln!(
                        "warning: no research files created for phase {phase_number} (this may be expected)"
                    );
                }

                cleanup.defuse();
            }

            PhaseStep::SpecExtract => {
                let product_spec = docs_dir.join("product-spec.md");
                let technical_spec = docs_dir.join("technical-spec.md");

                if !product_spec.exists() && !technical_spec.exists() {
                    // No specs to extract from; subsequent steps fall back to full injection.
                } else if specs_below_threshold(&product_spec, &technical_spec, config) {
                    crate::output::print_step(&format!(
                        "Skipping spec extraction for phase {phase_number} (specs below token threshold)"
                    ));
                } else {
                    crate::output::print_step(&format!(
                        "Extracting relevant specs for phase {phase_number} ({phase_title})"
                    ));
                    std::fs::create_dir_all(docs_dir.join("extracts"))
                        .context("creating docs/extracts directory")?;

                    let template_text = loader.load("spec_extract")?;
                    let target_file = format!("docs/extracts/phase-{padded}-specs.md");
                    let mut ctx = ContextBuilder::new();
                    ctx.add_file("phase spec", phase_spec_path)?;
                    ctx.add_file("product spec", &product_spec)?;
                    if technical_spec.exists() {
                        ctx.add_file("technical spec", &technical_spec)?;
                    }

                    let prompt = template::replace_vars(
                        &ctx.apply(&template_text),
                        &[
                            ("project_name", &config.project.name),
                            ("phase_number", &phase_number.to_string()),
                            ("phase_number_padded", padded),
                            ("target_file", &target_file),
                        ],
                    );
                    display.set_context_stats(ctx.total_tokens(), ctx.block_stats().len());

                    let result = invoke_sub_agent(
                        &prompt,
                        &config.models.spec_extract,
                        config.effort.spec_extract,
                        Some(PLANNING_TOOLS),
                        system_prompt_ref,
                        Some(project_dir),
                        &mut display,
                        &config.retry,
                        dry_run,
                    )
                    .await?;

                    accumulate_cost(state, phase_idx, result.cost_usd, "spec_extract");
                    state.save(state_path)?;

                    let extract_file = docs_dir.join(format!("extracts/phase-{padded}-specs.md"));
                    if !dry_run && !extract_file.exists() {
                        eprintln!(
                            "warning: spec extraction did not produce {target_file}; subsequent steps will use full specs"
                        );
                    }
                }
            }

            PhaseStep::Planning => {
                crate::output::print_step(&format!(
                    "Building implementation plan for phase {phase_number} ({phase_title})"
                ));
                let plans_dir = docs_dir.join("plans");
                std::fs::create_dir_all(&plans_dir).context("creating docs/plans directory")?;
                let mut cleanup = FileCleanupGuard::new(&plans_dir);

                let template_text = loader.load("phase_plan_generate")?;
                let target_file = format!("docs/plans/phase-{padded}.md");
                let mut ctx = ContextBuilder::new();
                add_reference_context(
                    &mut ctx,
                    phase_spec_path,
                    docs_dir,
                    padded,
                    project_dir,
                    phase_number,
                )?;
                if let Some(handoff_path) = find_most_recent_handoff(project_dir, phase_number) {
                    ctx.add_file("most recent handoff", &handoff_path)?;
                }
                let prompt = template::replace_vars(
                    &ctx.apply(&template_text),
                    &[
                        ("project_name", &config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("target_file", &target_file),
                    ],
                );
                display.set_context_stats(ctx.total_tokens(), ctx.block_stats().len());

                let result = invoke_sub_agent(
                    &prompt,
                    &config.models.planning,
                    config.effort.planning,
                    Some(PLANNING_TOOLS),
                    system_prompt_ref,
                    Some(project_dir),
                    &mut display,
                    &config.retry,
                    dry_run,
                )
                .await?;

                accumulate_cost(state, phase_idx, result.cost_usd, "planning");
                state.save(state_path)?;

                let plan_file = docs_dir.join(format!("plans/phase-{padded}.md"));
                if !dry_run && !plan_file.exists() {
                    bail!("sub agent did not create {target_file}");
                }

                cleanup.defuse();
            }

            PhaseStep::PlanReview {
                iteration: saved_iter,
            } => {
                let starting_iteration = if *saved_iter > 0 { *saved_iter + 1 } else { 1 };

                let review_template = loader.load("plan_review")?;
                let plan_file = docs_dir.join(format!("plans/phase-{padded}.md"));
                let review_prompt = template::replace_vars(
                    &review_template,
                    &[
                        ("project_name", &config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("target_file", &format!("docs/plans/phase-{padded}.md")),
                    ],
                );

                let plan_file_clone = plan_file.clone();
                let phase_spec_clone = phase_spec_path.to_path_buf();
                let docs_dir_clone = docs_dir.to_path_buf();
                let padded_clone = padded.to_string();
                let project_dir_clone = project_dir.to_path_buf();
                let mut review_params = ReviewParams {
                    config,
                    prompt_template: &review_prompt,
                    model: &config.models.review,
                    effort: config.effort.review,
                    max_iterations: config.review.max_iterations,
                    tools: Some(REVIEW_TOOLS),
                    system_prompt: system_prompt_ref,
                    cwd: Some(project_dir),
                    dry_run,
                    prior_findings: None,
                };
                let context_fn = || {
                    let path = plan_file_clone.clone();
                    let spec = phase_spec_clone.clone();
                    let docs = docs_dir_clone.clone();
                    let pad = padded_clone.clone();
                    let proj = project_dir_clone.clone();
                    async move {
                        let mut cb = ContextBuilder::new();
                        cb.add_file("phase plan", &path)?;
                        add_reference_context(&mut cb, &spec, &docs, &pad, &proj, phase_number)?;
                        Ok(cb)
                    }
                };

                let converged = run_review_loop(
                    &mut review_params,
                    config.effort.review,
                    starting_iteration,
                    &format!("Reviewing plan for phase {phase_number} ({phase_title})"),
                    &context_fn,
                    |iteration, cost| {
                        accumulate_cost(
                            state,
                            phase_idx,
                            cost,
                            &format!("plan review (iteration {iteration})"),
                        );
                        state.phases[phase_idx].current_step =
                            Some(PhaseStep::PlanReview { iteration });
                        state.save(state_path)
                    },
                )
                .await?;

                if !converged {
                    eprintln!(
                        "warning: plan review did not converge after {} iterations",
                        config.review.max_iterations
                    );
                    if config.interaction == InteractionMode::Milestones {
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
                let template_text = loader.load("execution")?;
                let plan_file = docs_dir.join(format!("plans/phase-{padded}.md"));
                let mut ctx = ContextBuilder::new();
                ctx.add_file("phase plan", &plan_file)?;
                add_reference_context(
                    &mut ctx,
                    phase_spec_path,
                    docs_dir,
                    padded,
                    project_dir,
                    phase_number,
                )?;
                if let Some(handoff_path) = find_most_recent_handoff(project_dir, phase_number) {
                    ctx.add_file("most recent handoff", &handoff_path)?;
                }
                let gate_commands_text = format_gate_commands(&config.gate_commands);
                let prompt = template::replace_vars(
                    &ctx.apply(&template_text),
                    &[
                        ("project_name", &config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("gate_commands", &gate_commands_text),
                    ],
                );
                display.set_context_stats(ctx.total_tokens(), ctx.block_stats().len());

                let result = invoke_sub_agent(
                    &prompt,
                    &config.models.execution,
                    config.effort.execution,
                    None,
                    system_prompt_ref,
                    Some(project_dir),
                    &mut display,
                    &config.retry,
                    dry_run,
                )
                .await?;

                accumulate_cost(state, phase_idx, result.cost_usd, "execution");
                state.save(state_path)?;

                if !result.result_text.is_empty() {
                    let summaries_dir = docs_dir.join("summaries");
                    std::fs::create_dir_all(&summaries_dir)
                        .context("creating docs/summaries directory")?;
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

                let code_review_template = loader.load("code_review")?;
                let plan_file = docs_dir.join(format!("plans/phase-{padded}.md"));
                let gate_commands_text = format_gate_commands(&config.gate_commands);
                let code_review_prompt = template::replace_vars(
                    &code_review_template,
                    &[
                        ("project_name", &config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("gate_commands", &gate_commands_text),
                    ],
                );

                let plan_file_clone = plan_file.clone();
                let phase_spec_clone = phase_spec_path.to_path_buf();
                let docs_dir_clone = docs_dir.to_path_buf();
                let padded_clone = padded.to_string();
                let project_dir_clone = project_dir.to_path_buf();
                let mut review_params = ReviewParams {
                    config,
                    prompt_template: &code_review_prompt,
                    model: &config.models.code_review,
                    effort: config.effort.code_review,
                    max_iterations: config.review.max_iterations,
                    tools: Some(CODE_REVIEW_TOOLS),
                    system_prompt: system_prompt_ref,
                    cwd: Some(project_dir),
                    dry_run,
                    prior_findings: None,
                };
                let context_fn = || {
                    let plan = plan_file_clone.clone();
                    let spec = phase_spec_clone.clone();
                    let docs = docs_dir_clone.clone();
                    let pad = padded_clone.clone();
                    let proj = project_dir_clone.clone();
                    async move {
                        let mut cb = ContextBuilder::new();
                        cb.add_file("phase plan", &plan)?;
                        add_reference_context(&mut cb, &spec, &docs, &pad, &proj, phase_number)?;
                        Ok(cb)
                    }
                };

                let converged = run_review_loop(
                    &mut review_params,
                    config.effort.code_review,
                    starting_iteration,
                    &format!("Reviewing code for phase {phase_number} ({phase_title})"),
                    &context_fn,
                    |iteration, cost| {
                        accumulate_cost(
                            state,
                            phase_idx,
                            cost,
                            &format!("code review (iteration {iteration})"),
                        );
                        state.phases[phase_idx].current_step =
                            Some(PhaseStep::CodeReview { iteration });
                        state.save(state_path)
                    },
                )
                .await?;

                if !converged {
                    eprintln!(
                        "warning: code review did not converge after {} iterations",
                        config.review.max_iterations
                    );
                    if config.interaction == InteractionMode::Milestones {
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
                let handoffs_dir = docs_dir.join("handoffs");
                std::fs::create_dir_all(&handoffs_dir)
                    .context("creating docs/handoffs directory")?;
                let mut cleanup = FileCleanupGuard::new(&handoffs_dir);

                let template_text = loader.load("handoff")?;
                let target_file = format!("docs/handoffs/phase-{padded}.md");
                let plan_file = docs_dir.join(format!("plans/phase-{padded}.md"));
                let execution_summary_path =
                    docs_dir.join(format!("summaries/phase-{padded}-execution.md"));
                let mut ctx = ContextBuilder::new();
                ctx.add_file("phase plan", &plan_file)?;
                if execution_summary_path.exists() {
                    ctx.add_file("execution summary", &execution_summary_path)?;
                }

                if let Some(commit) = pre_phase_commit {
                    let diff_stat = git_diff_stat(project_dir, commit).await;
                    if !diff_stat.is_empty() {
                        ctx.add_content("git diff summary", &diff_stat);
                    }
                }

                let prompt = template::replace_vars(
                    &ctx.apply(&template_text),
                    &[
                        ("project_name", &config.project.name),
                        ("phase_number", &phase_number.to_string()),
                        ("phase_number_padded", padded),
                        ("target_file", &target_file),
                    ],
                );
                display.set_context_stats(ctx.total_tokens(), ctx.block_stats().len());

                let result = invoke_sub_agent(
                    &prompt,
                    &config.models.review,
                    config.effort.review,
                    Some(REVIEW_TOOLS),
                    system_prompt_ref,
                    Some(project_dir),
                    &mut display,
                    &config.retry,
                    dry_run,
                )
                .await?;

                accumulate_cost(state, phase_idx, result.cost_usd, "handoff");
                state.save(state_path)?;

                let handoff_file = docs_dir.join(format!("handoffs/phase-{padded}.md"));
                if !dry_run && !handoff_file.exists() {
                    bail!("sub agent did not create {target_file}");
                }

                if !dry_run && execution_summary_path.exists() {
                    let _ = std::fs::remove_file(&execution_summary_path);
                }

                let extract_path = docs_dir.join(format!("extracts/phase-{padded}-specs.md"));
                if !dry_run && extract_path.exists() {
                    let _ = std::fs::remove_file(&extract_path);
                }

                cleanup.defuse();
            }

            PhaseStep::Commit => {
                crate::output::print_step(&format!(
                    "Committing changes for phase {phase_number} ({phase_title})"
                ));
                if config.git.auto_commit && pre_phase_commit.is_some() {
                    let title = &state.phases[phase_idx].title;
                    let message = format!("yoke: complete phase {padded} - {title}");
                    if let Err(e) = git_commit(project_dir, &message).await {
                        eprintln!("warning: git commit failed: {e}");
                    }
                }
            }
        }
    }

    state.phases[phase_idx].status = PhaseStatus::Completed;
    state.phases[phase_idx].current_step = None;
    state.phases[phase_idx].completed_at = Some(Utc::now());
    state.save(state_path)?;

    Ok(())
}

fn steps_from(starting: &PhaseStep) -> Vec<PhaseStep> {
    let all = [
        PhaseStep::SpecExtract,
        PhaseStep::Research,
        PhaseStep::Planning,
        PhaseStep::PlanReview { iteration: 0 },
        PhaseStep::Execution,
        PhaseStep::CodeReview { iteration: 0 },
        PhaseStep::Handoff,
        PhaseStep::Commit,
    ];

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

fn accumulate_cost(state: &mut YokeState, phase_idx: usize, cost: f64, step_label: &str) {
    state.phases[phase_idx].cost_usd += cost;
    state.phases[phase_idx].step_costs.push(StepCost {
        step: step_label.to_string(),
        cost_usd: cost,
    });
    state.total_cost_usd += cost;
}

fn specs_below_threshold(product_spec: &Path, technical_spec: &Path, config: &YokeConfig) -> bool {
    let threshold = config.context.spec_extract_threshold;
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

/// Add phase spec, spec extracts (or full specs), and research docs to a context builder.
/// Used by planning, plan review, code review, and execution steps.
fn add_reference_context(
    ctx: &mut ContextBuilder,
    phase_spec_path: &Path,
    docs_dir: &Path,
    padded: &str,
    project_dir: &Path,
    phase_number: usize,
) -> Result<()> {
    ctx.add_file("phase spec", phase_spec_path)?;
    add_spec_context(ctx, docs_dir, padded)?;
    for research_path in glob_research_files(project_dir, phase_number) {
        let label = research_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "research".to_string());
        ctx.add_file(&label, &research_path)?;
    }
    Ok(())
}

/// Inject spec context for steps that run after extraction.
/// Uses the phase-specific extract if available, otherwise falls back to full specs.
fn add_spec_context(ctx: &mut ContextBuilder, docs_dir: &Path, padded: &str) -> Result<()> {
    let extract_path = docs_dir.join(format!("extracts/phase-{padded}-specs.md"));
    if extract_path.exists() {
        ctx.add_file("relevant specifications", &extract_path)?;
        return Ok(());
    }
    add_specs_to_context(ctx, docs_dir)
}

fn add_specs_to_context(ctx: &mut ContextBuilder, docs_dir: &Path) -> Result<()> {
    let product_spec = docs_dir.join("product-spec.md");
    if product_spec.exists() {
        ctx.add_file("product spec", &product_spec)?;
    }
    let technical_spec = docs_dir.join("technical-spec.md");
    if technical_spec.exists() {
        ctx.add_file("technical spec", &technical_spec)?;
    }
    Ok(())
}

fn find_phase_spec(project_dir: &Path, phase_number: usize) -> Result<PathBuf> {
    let phases_dir = project_dir.join("docs/phases");
    if !phases_dir.exists() {
        bail!("docs/phases directory not found. Run `yoke plan` first to generate phase specs.");
    }

    let entries = std::fs::read_dir(&phases_dir)
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

    bail!("no phase spec file found for phase {phase_number} in docs/phases/")
}

pub fn find_most_recent_handoff(project_dir: &Path, before_phase: usize) -> Option<PathBuf> {
    let handoffs_dir = project_dir.join("docs/handoffs");
    if !handoffs_dir.exists() {
        return None;
    }

    let entries = std::fs::read_dir(&handoffs_dir).ok()?;
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

pub fn glob_research_files(project_dir: &Path, phase_number: usize) -> Vec<PathBuf> {
    let research_dir = project_dir.join("docs/research");
    if !research_dir.exists() {
        return Vec::new();
    }

    let prefix = format!("phase-{:03}-", phase_number);
    let entries = match std::fs::read_dir(&research_dir) {
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

pub async fn capture_git_head(project_dir: &Path) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(project_dir)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if hash.is_empty() {
        return None;
    }
    Some(hash)
}

async fn git_diff_stat(project_dir: &Path, from_commit: &str) -> String {
    let output = tokio::process::Command::new("git")
        .args(["diff", &format!("{from_commit}..HEAD"), "--stat"])
        .current_dir(project_dir)
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}

async fn git_commit(project_dir: &Path, message: &str) -> Result<()> {
    let add = tokio::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(project_dir)
        .output()
        .await
        .context("running git add")?;

    if !add.status.success() {
        bail!("git add failed: {}", String::from_utf8_lossy(&add.stderr));
    }

    let commit = tokio::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(project_dir)
        .output()
        .await
        .context("running git commit")?;

    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr);
        if stderr.contains("nothing to commit") {
            return Ok(());
        }
        bail!("git commit failed: {stderr}");
    }

    Ok(())
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
        let dir = std::env::temp_dir().join("yoke_handoff_test");
        let handoffs = dir.join("docs/handoffs");
        let _ = std::fs::create_dir_all(&handoffs);

        std::fs::write(handoffs.join("phase-001.md"), "h1").unwrap();
        std::fs::write(handoffs.join("phase-002.md"), "h2").unwrap();
        std::fs::write(handoffs.join("phase-004.md"), "h4").unwrap();

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
        let dir = std::env::temp_dir().join("yoke_research_glob_test");
        let research = dir.join("docs/research");
        let _ = std::fs::create_dir_all(&research);

        std::fs::write(research.join("phase-001-findings.md"), "f1").unwrap();
        std::fs::write(research.join("phase-001-api-notes.md"), "f2").unwrap();
        std::fs::write(research.join("phase-002-findings.md"), "f3").unwrap();
        std::fs::write(research.join("unrelated.md"), "x").unwrap();

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
        let steps = steps_from(&PhaseStep::SpecExtract);
        assert_eq!(steps.len(), 8);
        assert_eq!(step_ordinal(&steps[0]), 0);
        assert_eq!(step_ordinal(&steps[7]), 7);
    }

    #[test]
    fn steps_from_research_skips_spec_extract() {
        let steps = steps_from(&PhaseStep::Research);
        assert_eq!(steps.len(), 7);
        assert_eq!(step_ordinal(&steps[0]), 1);
    }

    #[test]
    fn steps_from_execution_returns_tail() {
        let steps = steps_from(&PhaseStep::Execution);
        assert_eq!(steps.len(), 4);
        assert_eq!(step_ordinal(&steps[0]), 4);
    }

    #[test]
    fn steps_from_commit_returns_one() {
        let steps = steps_from(&PhaseStep::Commit);
        assert_eq!(steps.len(), 1);
        assert_eq!(step_ordinal(&steps[0]), 7);
    }

    #[test]
    fn steps_from_plan_review_preserves_iteration() {
        let steps = steps_from(&PhaseStep::PlanReview { iteration: 3 });
        assert_eq!(steps[0], PhaseStep::PlanReview { iteration: 3 });
    }

    #[test]
    fn steps_from_code_review_preserves_iteration() {
        let steps = steps_from(&PhaseStep::CodeReview { iteration: 2 });
        assert_eq!(steps[0], PhaseStep::CodeReview { iteration: 2 });
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
