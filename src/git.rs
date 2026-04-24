use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::{GitConfig, YokeConfig};
use crate::intent::IntentState;

pub async fn capture_head(dir: &Path) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(dir)
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

pub async fn diff_stat(dir: &Path, from_commit: &str) -> String {
    let output = tokio::process::Command::new("git")
        .args(["diff", &format!("{from_commit}..HEAD"), "--stat"])
        .current_dir(dir)
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}

pub async fn commit_all(dir: &Path, message: &str) -> Result<()> {
    let add = tokio::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .await
        .context("running git add")?;

    if !add.status.success() {
        bail!("git add failed: {}", String::from_utf8_lossy(&add.stderr));
    }

    let commit = tokio::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir)
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

pub async fn current_branch(dir: &Path) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()
        .await
        .context("running git rev-parse")?;

    if !output.status.success() {
        bail!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn create_branch(dir: &Path, name: &str) -> Result<()> {
    let output = tokio::process::Command::new("git")
        .args(["branch", name])
        .current_dir(dir)
        .output()
        .await
        .context("running git branch")?;

    if !output.status.success() {
        bail!(
            "git branch failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub async fn checkout_branch(dir: &Path, name: &str) -> Result<()> {
    let output = tokio::process::Command::new("git")
        .args(["checkout", name])
        .current_dir(dir)
        .output()
        .await
        .context("running git checkout")?;

    if !output.status.success() {
        bail!(
            "git checkout failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub async fn create_worktree(
    project_dir: &Path,
    worktree_path: &Path,
    branch: &str,
) -> Result<PathBuf> {
    let output = tokio::process::Command::new("git")
        .args([
            "worktree",
            "add",
            &worktree_path.display().to_string(),
            branch,
        ])
        .current_dir(project_dir)
        .output()
        .await
        .context("running git worktree add")?;

    if !output.status.success() {
        bail!(
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(worktree_path.to_path_buf())
}

pub async fn remove_worktree(project_dir: &Path, worktree_path: &Path) -> Result<()> {
    let output = tokio::process::Command::new("git")
        .args([
            "worktree",
            "remove",
            &worktree_path.display().to_string(),
            "--force",
        ])
        .current_dir(project_dir)
        .output()
        .await
        .context("running git worktree remove")?;

    if !output.status.success() {
        bail!(
            "git worktree remove failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub fn resolve_worktree_dir(
    config: &YokeConfig,
    intent: &IntentState,
    project_dir: &Path,
) -> PathBuf {
    let template = config
        .worktrees
        .directory
        .replace("{project}", &config.project.name);
    project_dir.join(template).join(intent.dir_name())
}

pub fn intent_branch_name(config: &GitConfig, intent: &IntentState) -> String {
    format!("{}/{}-{}", config.branch_prefix, intent.id, intent.slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GitConfig, YokeConfig};
    use crate::intent::{Classification, Depth, IntentState};

    #[test]
    fn intent_branch_name_format() {
        let config = GitConfig::default();
        let intent = IntentState::new(
            3,
            "Auth feature".to_string(),
            "Add auth".to_string(),
            Classification::Feature,
            Depth::Light,
        );
        assert_eq!(
            intent_branch_name(&config, &intent),
            "yoke/i-003-auth-feature"
        );
    }

    #[test]
    fn intent_branch_name_custom_prefix() {
        let config = GitConfig {
            auto_commit: true,
            branch_prefix: "feat".to_string(),
        };
        let intent = IntentState::new(
            1,
            "Initial build".to_string(),
            "Build it".to_string(),
            Classification::Build,
            Depth::Full,
        );
        assert_eq!(
            intent_branch_name(&config, &intent),
            "feat/i-001-initial-build"
        );
    }

    fn test_config(name: &str) -> YokeConfig {
        let toml_str = format!(
            r#"
[project]
name = "{name}"
"#
        );
        toml::from_str(&toml_str).unwrap()
    }

    #[test]
    fn resolve_worktree_dir_default_template() {
        let config = test_config("myproject");
        let intent = IntentState::new(
            2,
            "Auth feature".to_string(),
            "Add auth".to_string(),
            Classification::Feature,
            Depth::Light,
        );
        let project_dir = Path::new("/home/user/myproject");
        let result = resolve_worktree_dir(&config, &intent, project_dir);
        assert_eq!(
            result,
            PathBuf::from("/home/user/myproject/../myproject-worktrees/i-002-auth-feature")
        );
    }

    #[test]
    fn resolve_worktree_dir_absolute_template() {
        let toml_str = r#"
[project]
name = "app"

[worktrees]
directory = "/tmp/{project}-wt"
"#;
        let config: YokeConfig = toml::from_str(toml_str).unwrap();
        let intent = IntentState::new(
            5,
            "Fix login".to_string(),
            "Fix it".to_string(),
            Classification::Fix,
            Depth::Minimal,
        );
        let project_dir = Path::new("/home/user/app");
        let result = resolve_worktree_dir(&config, &intent, project_dir);
        assert_eq!(result, PathBuf::from("/tmp/app-wt/i-005-fix-login"));
    }

    #[test]
    fn resolve_worktree_dir_relative_template() {
        let config = test_config("proj");
        let intent = IntentState::new(
            1,
            "Build".to_string(),
            "Build it".to_string(),
            Classification::Build,
            Depth::Full,
        );
        let project_dir = Path::new("/workspace/proj");
        let result = resolve_worktree_dir(&config, &intent, project_dir);
        assert_eq!(
            result,
            PathBuf::from("/workspace/proj/../proj-worktrees/i-001-build")
        );
    }
}
