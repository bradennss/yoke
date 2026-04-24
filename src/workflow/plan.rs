use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::config::{InteractionMode, YokeConfig};
use crate::output::StreamDisplay;
use crate::state::{PhaseState, PhaseStatus, PlanStep, StageStatus, YokeState, plan_step_ordinal};
use crate::template;
use crate::workflow::cleanup::{FileCleanupGuard, SingleFileGuard};
use crate::workflow::context::ContextBuilder;
use crate::workflow::review::{ReviewParams, run_review_loop};
use crate::workflow::{build_system_prompt, invoke_sub_agent, prompt_loader};

const GENERATION_TOOLS: &str = "Read,Write,Edit,Glob,Grep,Bash";
const REVIEW_TOOLS: &str = "Read,Write,Edit,Glob,Grep";

pub async fn run_plan(
    project_dir: &Path,
    config: &YokeConfig,
    state: &mut YokeState,
    dry_run: bool,
) -> Result<()> {
    let docs_dir = project_dir.join("docs");
    let product_spec_path = docs_dir.join("product-spec.md");
    let technical_spec_path = docs_dir.join("technical-spec.md");

    if !product_spec_path.exists() {
        bail!("docs/product-spec.md not found. Run `yoke spec` first to generate specifications.");
    }
    if !technical_spec_path.exists() {
        bail!(
            "docs/technical-spec.md not found. Run `yoke spec` first to generate specifications."
        );
    }

    let loader = prompt_loader(project_dir);
    let system_prompt = build_system_prompt(config, &loader)?;
    let state_path = project_dir.join(".yoke/state.json");

    let starting_step = state.plan_step.clone().unwrap_or(PlanStep::PlanGeneration);
    let start_ord = plan_step_ordinal(&starting_step);

    state.plan_status = StageStatus::InProgress;
    state.save(&state_path)?;

    let plan_path = docs_dir.join("plan.md");
    let phases_dir = docs_dir.join("phases");

    if start_ord <= plan_step_ordinal(&PlanStep::PlanGeneration) {
        crate::output::print_step("Generating implementation plan and breaking into phases");
        state.plan_step = Some(PlanStep::PlanGeneration);
        state.save(&state_path)?;

        let mut plan_cleanup = SingleFileGuard::new(&plan_path);
        let mut phases_cleanup = FileCleanupGuard::new(&phases_dir);

        let plan_template = loader.load("plan_generate")?;
        let mut ctx = ContextBuilder::new();
        ctx.add_file("product-spec.md", &product_spec_path)?;
        ctx.add_file("technical-spec.md", &technical_spec_path)?;
        let prompt = template::replace_vars(
            &ctx.apply(&plan_template),
            &[
                ("project_name", &config.project.name),
                ("target_file", "docs/plan.md"),
            ],
        );

        let mut display = StreamDisplay::new();
        display.set_context_stats(ctx.total_tokens(), ctx.block_stats().len());
        let result = invoke_sub_agent(
            &prompt,
            &config.models.planning,
            config.effort.planning,
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

        state.plan_cost_usd += result.cost_usd;
        state.total_cost_usd += result.cost_usd;
        state.save(&state_path)?;

        if !plan_path.exists() {
            bail!("sub agent did not create docs/plan.md");
        }

        plan_cleanup.defuse();
        phases_cleanup.defuse();
    }

    // Step 1: Verify phase files exist
    if start_ord <= plan_step_ordinal(&PlanStep::PhaseVerification) {
        state.plan_step = Some(PlanStep::PhaseVerification);
        state.save(&state_path)?;

        let phase_files = glob_phase_files(&phases_dir)?;
        if phase_files.is_empty() {
            bail!("no phase files found in docs/phases/. Expected files matching NNN-title.md.");
        }
    }

    if start_ord <= plan_step_ordinal(&PlanStep::PlanReview { iteration: 0 }) {
        let starting_iteration = match &starting_step {
            PlanStep::PlanReview { iteration } if *iteration > 0 => *iteration + 1,
            _ => 1,
        };

        state.plan_step = Some(PlanStep::PlanReview {
            iteration: starting_iteration.saturating_sub(1),
        });
        state.save(&state_path)?;

        let review_template = loader.load("plan_review")?;
        let review_prompt = template::replace_vars(
            &review_template,
            &[
                ("project_name", &config.project.name),
                ("target_file", "docs/plan.md"),
            ],
        );

        let plan_path_clone = plan_path.clone();
        let mut review_params = ReviewParams {
            config,
            prompt_template: &review_prompt,
            model: &config.models.review,
            effort: config.effort.review,
            max_iterations: config.review.max_iterations,
            tools: Some(REVIEW_TOOLS),
            system_prompt: Some(system_prompt.as_str()),
            cwd: Some(project_dir),
            dry_run,
            prior_findings: None,
        };
        let context_fn = || {
            let path = plan_path_clone.clone();
            async move {
                let mut cb = ContextBuilder::new();
                cb.add_file("plan.md", &path)?;
                Ok(cb)
            }
        };

        let converged = run_review_loop(
            &mut review_params,
            config.effort.review,
            starting_iteration,
            "Reviewing implementation plan",
            &context_fn,
            |iteration, cost| {
                state.plan_cost_usd += cost;
                state.total_cost_usd += cost;
                state.plan_step = Some(PlanStep::PlanReview { iteration });
                state.save(&state_path)
            },
        )
        .await?;

        if !converged {
            eprintln!(
                "warning: plan review did not converge after {} iterations",
                config.review.max_iterations
            );
        }
    }

    // Re-glob after review (reviewer may have added or renamed phase files)
    let phase_files = glob_phase_files(&phases_dir)?;
    let mut phases: Vec<PhaseState> = phase_files
        .into_iter()
        .map(|(number, title)| PhaseState {
            number,
            title,
            status: PhaseStatus::Pending,
            current_step: None,
            cost_usd: 0.0,
            step_costs: Vec::new(),
            pre_phase_commit: None,
            started_at: None,
            completed_at: None,
        })
        .collect();
    phases.sort_by_key(|p| p.number);

    state.phases = phases;
    state.plan_status = StageStatus::Complete;
    state.plan_step = None;
    state.save(&state_path)?;

    if config.interaction == InteractionMode::Milestones {
        println!("Plan complete. Review the plan at docs/plan.md, then re-run to continue.");
    }

    Ok(())
}

fn glob_phase_files(phases_dir: &Path) -> Result<Vec<(usize, String)>> {
    if !phases_dir.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    let entries = std::fs::read_dir(phases_dir)
        .with_context(|| format!("reading directory {}", phases_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if let Some(parsed) = parse_phase_filename(&name) {
            results.push(parsed);
        }
    }

    results.sort_by_key(|(n, _)| *n);
    Ok(results)
}

pub fn parse_phase_filename(filename: &str) -> Option<(usize, String)> {
    let stem = filename.strip_suffix(".md")?;

    let dash_pos = stem.find('-')?;
    let number_str = &stem[..dash_pos];
    let title_str = &stem[dash_pos + 1..];

    if number_str.is_empty() || title_str.is_empty() {
        return None;
    }

    if !number_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let number: usize = number_str.parse().ok()?;
    let title = title_str.replace('-', " ");

    Some((number, title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_phase_filename() {
        let result = parse_phase_filename("001-setup.md");
        assert_eq!(result, Some((1, "setup".to_string())));
    }

    #[test]
    fn parse_multi_word_title() {
        let result = parse_phase_filename("003-core-data-models.md");
        assert_eq!(result, Some((3, "core data models".to_string())));
    }

    #[test]
    fn parse_high_number() {
        let result = parse_phase_filename("042-final-integration.md");
        assert_eq!(result, Some((42, "final integration".to_string())));
    }

    #[test]
    fn parse_single_digit() {
        let result = parse_phase_filename("1-init.md");
        assert_eq!(result, Some((1, "init".to_string())));
    }

    #[test]
    fn reject_no_extension() {
        assert_eq!(parse_phase_filename("001-setup"), None);
    }

    #[test]
    fn reject_wrong_extension() {
        assert_eq!(parse_phase_filename("001-setup.txt"), None);
    }

    #[test]
    fn reject_no_dash() {
        assert_eq!(parse_phase_filename("001setup.md"), None);
    }

    #[test]
    fn reject_no_number() {
        assert_eq!(parse_phase_filename("-setup.md"), None);
    }

    #[test]
    fn reject_no_title() {
        assert_eq!(parse_phase_filename("001-.md"), None);
    }

    #[test]
    fn reject_non_numeric_prefix() {
        assert_eq!(parse_phase_filename("abc-setup.md"), None);
    }

    #[test]
    fn reject_empty_string() {
        assert_eq!(parse_phase_filename(""), None);
    }

    #[test]
    fn reject_just_md() {
        assert_eq!(parse_phase_filename(".md"), None);
    }

    #[test]
    fn glob_phase_files_nonexistent_dir() {
        let result = glob_phase_files(Path::new("/tmp/yoke_nonexistent_phases_dir_12345"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn glob_phase_files_with_valid_files() {
        let dir = std::env::temp_dir().join("yoke_glob_phase_test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("001-setup.md"), "phase 1").unwrap();
        std::fs::write(dir.join("002-core.md"), "phase 2").unwrap();
        std::fs::write(dir.join("README.md"), "not a phase").unwrap();

        let result = glob_phase_files(&dir).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], (1, "setup".to_string()));
        assert_eq!(result[1], (2, "core".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn glob_phase_files_sorted_by_number() {
        let dir = std::env::temp_dir().join("yoke_glob_phase_sort_test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("003-last.md"), "").unwrap();
        std::fs::write(dir.join("001-first.md"), "").unwrap();
        std::fs::write(dir.join("002-middle.md"), "").unwrap();

        let result = glob_phase_files(&dir).unwrap();
        assert_eq!(result[0].0, 1);
        assert_eq!(result[1].0, 2);
        assert_eq!(result[2].0, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
