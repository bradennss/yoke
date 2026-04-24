use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::config::InteractionMode;
use crate::output::StreamDisplay;
use crate::state::{PhaseState, PhaseStatus, PlanStep, StageStatus, plan_step_ordinal};
use crate::template;
use crate::workflow::IntentContext;
use crate::workflow::cleanup::{FileCleanupGuard, SingleFileGuard};
use crate::workflow::context::ContextBuilder;
use crate::workflow::review::{ReviewParams, run_review_loop};

const GENERATION_TOOLS: &str = "Read,Write,Edit,Glob,Grep,Bash";
const REVIEW_TOOLS: &str = "Read,Write,Edit,Glob,Grep";

pub async fn run_plan(ctx: &mut IntentContext<'_>) -> Result<()> {
    let product_spec_path = ctx.product_spec_path();
    let technical_spec_path = ctx.technical_spec_path();

    if !product_spec_path.exists() {
        bail!(
            "{} not found. Run specs first to generate specifications.",
            product_spec_path.display()
        );
    }
    if !technical_spec_path.exists() {
        bail!(
            "{} not found. Run specs first to generate specifications.",
            technical_spec_path.display()
        );
    }

    let starting_step = ctx
        .intent
        .plan_step
        .clone()
        .unwrap_or(PlanStep::PlanGeneration);
    let start_ord = plan_step_ordinal(&starting_step);

    ctx.intent.plan_status = StageStatus::InProgress;
    ctx.save_intent()?;

    let plan_path = ctx.plan_path();
    let phases_dir = ctx.phases_dir();

    if start_ord <= plan_step_ordinal(&PlanStep::PlanGeneration) {
        crate::output::print_step("Generating implementation plan and breaking into phases");
        ctx.intent.plan_step = Some(PlanStep::PlanGeneration);
        ctx.save_intent()?;

        let mut plan_cleanup = SingleFileGuard::new(&plan_path);
        let mut phases_cleanup = FileCleanupGuard::new(&phases_dir);

        let plan_template = ctx.loader.load("plan_generate")?;
        let mut cb = ContextBuilder::new();
        cb.add_file("product-spec.md", &product_spec_path)?;
        cb.add_file("technical-spec.md", &technical_spec_path)?;
        let prompt = template::replace_vars(
            &cb.apply(&plan_template),
            &[
                ("project_name", &ctx.config.project.name),
                ("target_file", &plan_path.display().to_string()),
            ],
        );

        let mut display = StreamDisplay::new();
        display.set_context_stats(cb.total_tokens(), cb.block_stats().len());
        let result = super::invoke_sub_agent(
            &prompt,
            &ctx.config.plan.model,
            ctx.config.plan.effort,
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

        ctx.intent.plan_cost_usd += result.cost_usd;
        ctx.intent.total_cost_usd += result.cost_usd;
        ctx.save_intent()?;

        if !plan_path.exists() {
            bail!("sub agent did not create {}", plan_path.display());
        }

        plan_cleanup.defuse();
        phases_cleanup.defuse();
    }

    if start_ord <= plan_step_ordinal(&PlanStep::PhaseVerification) {
        ctx.intent.plan_step = Some(PlanStep::PhaseVerification);
        ctx.save_intent()?;

        let phase_files = glob_phase_files(&phases_dir)?;
        if phase_files.is_empty() {
            bail!(
                "no phase files found in {}. Expected files matching NNN-title.md.",
                phases_dir.display()
            );
        }
    }

    if start_ord <= plan_step_ordinal(&PlanStep::PlanReview { iteration: 0 }) {
        let starting_iteration = match &starting_step {
            PlanStep::PlanReview { iteration } if *iteration > 0 => *iteration + 1,
            _ => 1,
        };

        ctx.intent.plan_step = Some(PlanStep::PlanReview {
            iteration: starting_iteration.saturating_sub(1),
        });
        ctx.save_intent()?;

        let review_template = ctx.loader.load("plan_review")?;
        let review_prompt = template::replace_vars(
            &review_template,
            &[
                ("project_name", &ctx.config.project.name),
                ("target_file", &plan_path.display().to_string()),
            ],
        );

        let plan_path_clone = plan_path.clone();
        let system_prompt_clone = ctx.system_prompt.clone();
        let work_dir_clone = ctx.work_dir.clone();
        let mut review_params = ReviewParams {
            config: ctx.config,
            prompt_template: &review_prompt,
            model: &ctx.config.plan.review_model,
            effort: ctx.config.plan.review_effort,
            max_iterations: ctx.config.plan.max_review_iterations,
            tools: Some(REVIEW_TOOLS),
            system_prompt: Some(&system_prompt_clone),
            cwd: Some(&work_dir_clone),
            dry_run: ctx.dry_run,
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
            ctx.config.plan.review_effort,
            starting_iteration,
            "Reviewing implementation plan",
            &context_fn,
            |iteration, cost| {
                ctx.intent.plan_cost_usd += cost;
                ctx.intent.total_cost_usd += cost;
                ctx.intent.plan_step = Some(PlanStep::PlanReview { iteration });
                ctx.save_intent()
            },
        )
        .await?;

        if !converged {
            eprintln!(
                "warning: plan review did not converge after {} iterations",
                ctx.config.plan.max_review_iterations
            );
        }
    }

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

    ctx.intent.phases = phases;
    ctx.intent.plan_status = StageStatus::Complete;
    ctx.intent.plan_step = None;
    ctx.save_intent()?;

    if ctx.config.interaction == InteractionMode::Milestones {
        println!(
            "Plan complete. Review the plan at {}, then re-run to continue.",
            plan_path.display()
        );
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
