mod init;
mod plan;
mod run;
mod spec;
mod status;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::claude;
use crate::claude::ClaudeInvocation;
use crate::output::StreamDisplay;

#[derive(Parser)]
#[command(
    name = "yoke",
    about = "Orchestrate Claude Code through structured development workflows"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Show what would be done without invoking Claude
    #[arg(long, global = true)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a raw prompt through Claude Code (debug command)
    Exec {
        /// The prompt to send to Claude
        prompt: String,

        /// Model to use (e.g., opus, sonnet, haiku)
        #[arg(long)]
        model: Option<String>,
    },

    /// Initialize a new yoke project in the current directory
    Init,

    /// Generate product and technical specs from a description
    Spec {
        /// Rough description of the project
        description: Option<String>,

        /// Read description from a file
        #[arg(long)]
        file: Option<PathBuf>,
    },

    /// Generate the implementation plan from specs
    Plan,

    /// Execute a phase of the implementation plan
    Run {
        /// Phase number to execute (defaults to next pending phase)
        phase: Option<usize>,

        /// Restart from a specific step
        #[arg(long)]
        from: Option<String>,
    },

    /// Show the current project status
    Status,
}

pub async fn run(cli: Cli) -> Result<()> {
    let dry_run = cli.dry_run;
    match cli.command {
        Command::Exec { prompt, model } => exec(&prompt, model.as_deref(), dry_run).await,
        Command::Init => {
            let cwd = std::env::current_dir()?;
            init::run(&cwd)
        }
        Command::Spec { description, file } => {
            let cwd = std::env::current_dir()?;
            spec::run(&cwd, description, file, dry_run).await
        }
        Command::Plan => {
            let cwd = std::env::current_dir()?;
            plan::run(&cwd, dry_run).await
        }
        Command::Run { phase, from } => {
            let cwd = std::env::current_dir()?;
            run::run(&cwd, phase, from, dry_run).await
        }
        Command::Status => {
            let cwd = std::env::current_dir()?;
            status::run(&cwd)
        }
    }
}

async fn exec(prompt: &str, model: Option<&str>, dry_run: bool) -> Result<()> {
    if dry_run {
        let display_prompt: String = prompt.chars().take(500).collect();
        eprintln!("[dry run] prompt: {display_prompt}");
        if let Some(m) = model {
            eprintln!("[dry run] model: {m}");
        }
        return Ok(());
    }

    claude::verify_available().await?;

    let mut invocation = ClaudeInvocation::new(prompt);
    if let Some(m) = model {
        invocation = invocation.model(m);
    }

    let mut process = invocation.spawn()?;
    let mut display = StreamDisplay::new();

    while let Some(event) = process.next_event().await {
        display.handle_event(&event);
    }

    let result = process.finish().await?;
    if let Some(code) = result.exit_code
        && code != 0
    {
        std::process::exit(code);
    }

    Ok(())
}
