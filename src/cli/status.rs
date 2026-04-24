use std::path::Path;

use anyhow::Result;
use crossterm::style::{Stylize, style};

use crate::intent::IntentStatus;
use crate::intent::store::IntentStore;
use crate::state::{PhaseStatus, StageStatus};

pub fn run(project_dir: &Path) -> Result<()> {
    let store = IntentStore::new(project_dir);
    let intents = store.list()?;

    if intents.is_empty() {
        println!(
            "{}",
            style("No intents found. Run `yoke new` to create one.").dim()
        );
        return Ok(());
    }

    let mut project_cost = 0.0;

    for intent in &intents {
        let status_styled = match intent.status {
            IntentStatus::Pending => style(format!("{}", intent.status)).dim(),
            IntentStatus::InProgress => style(format!("{}", intent.status)).yellow(),
            IntentStatus::Completed => style(format!("{}", intent.status)).green(),
            IntentStatus::Failed => style(format!("{}", intent.status)).red(),
            IntentStatus::Blocked => style(format!("{}", intent.status)).magenta(),
        };

        println!(
            "\n{} {} [{}] ({}, {})",
            style(&intent.id).bold(),
            intent.title,
            status_styled,
            intent.classification,
            intent.depth,
        );

        print_stage("  Spec", &intent.spec_status);
        if let Some(ref step) = intent.spec_step {
            println!("    current step: {step}");
        }
        if intent.spec_cost_usd > 0.0 {
            println!("    cost: ${:.4}", intent.spec_cost_usd);
        }

        print_stage("  Plan", &intent.plan_status);
        if let Some(ref step) = intent.plan_step {
            println!("    current step: {step}");
        }
        if intent.plan_cost_usd > 0.0 {
            println!("    cost: ${:.4}", intent.plan_cost_usd);
        }

        if !intent.phases.is_empty() {
            for phase in &intent.phases {
                let phase_status = match phase.status {
                    PhaseStatus::Pending => style(format!("{}", phase.status)).dim(),
                    PhaseStatus::InProgress => style(format!("{}", phase.status)).yellow(),
                    PhaseStatus::Completed => style(format!("{}", phase.status)).green(),
                    PhaseStatus::Failed => style(format!("{}", phase.status)).red(),
                };

                println!(
                    "    Phase {}: {} [{}]",
                    phase.number, phase.title, phase_status
                );

                if let Some(ref step) = phase.current_step {
                    println!("      current step: {step}");
                }

                if phase.cost_usd > 0.0 {
                    println!("      cost: ${:.4}", phase.cost_usd);
                }

                if !phase.step_costs.is_empty() {
                    for sc in &phase.step_costs {
                        println!("        {}: ${:.4}", sc.step, sc.cost_usd);
                    }
                }
            }
        }

        if intent.total_cost_usd > 0.0 {
            println!("  Intent cost: ${:.4}", intent.total_cost_usd);
        }

        project_cost += intent.total_cost_usd;
    }

    println!("\nProject total cost: ${:.4}", project_cost);

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
