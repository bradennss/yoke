mod discover;
mod init;
mod knowledge;
mod list;
mod new;
mod run;
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

    /// Discover and generate specs from an existing codebase
    Discover,

    /// Create a new intent
    New {
        /// Description of the intent
        description: Option<String>,

        /// Read description from a file
        #[arg(long)]
        file: Option<PathBuf>,

        /// Classification (build, feature, fix, refactor, maintenance)
        #[arg(long, value_name = "CLASS")]
        class: Option<String>,

        /// Depth override (full, light, minimal)
        #[arg(long)]
        depth: Option<String>,

        /// Parent intent id
        #[arg(long)]
        parent: Option<String>,

        /// Intent ids that block this intent
        #[arg(long, num_args = 1..)]
        blocked_by: Option<Vec<String>>,

        /// Skip confirmation prompts
        #[arg(long, short)]
        yes: bool,
    },

    /// List all intents
    List,

    /// Run the next pending intent (or a specific intent/phase)
    Run {
        /// Intent id (e.g., i-003)
        intent: Option<String>,

        /// Run a specific phase number
        #[arg(long)]
        phase: Option<usize>,

        /// Restart from a specific step within a phase
        #[arg(long)]
        from: Option<String>,
    },

    /// Show the current project status
    Status,

    /// Display the project knowledge base
    Knowledge,
}

pub async fn run(cli: Cli) -> Result<()> {
    let dry_run = cli.dry_run;
    match cli.command {
        Command::Exec { prompt, model } => exec(&prompt, model.as_deref(), dry_run).await,
        Command::Init => {
            let cwd = std::env::current_dir()?;
            init::run(&cwd)
        }
        Command::Discover => {
            let cwd = std::env::current_dir()?;
            discover::run(&cwd, dry_run).await
        }
        Command::New {
            description,
            file,
            class,
            depth,
            parent,
            blocked_by,
            yes,
        } => {
            let cwd = std::env::current_dir()?;
            new::run(
                &cwd,
                description,
                file,
                class,
                depth,
                parent,
                blocked_by,
                yes,
                dry_run,
            )
            .await
        }
        Command::List => {
            let cwd = std::env::current_dir()?;
            list::run(&cwd)
        }
        Command::Run {
            intent,
            phase,
            from,
        } => {
            let cwd = std::env::current_dir()?;
            run::run(&cwd, intent, phase, from, dry_run).await
        }
        Command::Status => {
            let cwd = std::env::current_dir()?;
            status::run(&cwd)
        }
        Command::Knowledge => {
            let cwd = std::env::current_dir()?;
            knowledge::run(&cwd)
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
