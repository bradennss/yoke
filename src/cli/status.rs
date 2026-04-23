use std::path::Path;

use anyhow::{Context, Result};
use crossterm::style::{Stylize, style};

use crate::state::{PhaseStatus, StageStatus, YokeState};

pub fn run(project_dir: &Path) -> Result<()> {
    let state_path = project_dir.join(".yoke/state.json");
    let state =
        YokeState::load(&state_path).context("could not load state; have you run `yoke init`?")?;

    print_stage("Spec", &state.spec_status);
    if let Some(ref step) = state.spec_step {
        println!("  current step: {step}");
    }
    if state.spec_cost_usd > 0.0 {
        println!("  cost: ${:.4}", state.spec_cost_usd);
    }

    print_stage("Plan", &state.plan_status);
    if let Some(ref step) = state.plan_step {
        println!("  current step: {step}");
    }
    if state.plan_cost_usd > 0.0 {
        println!("  cost: ${:.4}", state.plan_cost_usd);
    }

    if state.phases.is_empty() {
        println!("\n{}", style("No phases defined yet.").dim());
    } else {
        println!();
        for phase in &state.phases {
            let status_styled = match phase.status {
                PhaseStatus::Pending => style(format!("{}", phase.status)).dim(),
                PhaseStatus::InProgress => style(format!("{}", phase.status)).yellow(),
                PhaseStatus::Completed => style(format!("{}", phase.status)).green(),
                PhaseStatus::Failed => style(format!("{}", phase.status)).red(),
            };

            println!(
                "  Phase {}: {} [{}]",
                phase.number, phase.title, status_styled
            );

            if let Some(ref step) = phase.current_step {
                println!("    current step: {step}");
            }

            if phase.cost_usd > 0.0 {
                println!("    cost: ${:.4}", phase.cost_usd);
            }

            if !phase.step_costs.is_empty() {
                for sc in &phase.step_costs {
                    println!("      {}: ${:.4}", sc.step, sc.cost_usd);
                }
            }
        }
    }

    println!("\nTotal cost: ${:.4}", state.total_cost_usd);

    Ok(())
}

fn print_stage(label: &str, status: &StageStatus) {
    let styled = match status {
        StageStatus::Pending => style(format!("{status}")).dim(),
        StageStatus::InProgress => style(format!("{status}")).yellow(),
        StageStatus::Complete => style(format!("{status}")).green(),
    };
    println!("{label}: {styled}");
}
