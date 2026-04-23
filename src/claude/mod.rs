pub mod process;
pub mod stream;
pub mod types;

pub use process::{ClaudeInvocation, ClaudeProcess, ProcessResult};
pub use types::StreamEvent;

pub async fn verify_available() -> anyhow::Result<()> {
    let result = tokio::process::Command::new("claude")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => anyhow::bail!(
            "claude CLI exited with code {}",
            status.code().unwrap_or(-1)
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!("claude CLI not found in PATH")
        }
        Err(e) => anyhow::bail!("failed to run claude CLI: {e}"),
    }
}
