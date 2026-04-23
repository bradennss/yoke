use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = yoke::cli::Cli::parse();

    tokio::select! {
        result = yoke::cli::run(cli) => result,
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\ninterrupted");
            std::process::exit(130);
        }
    }
}
