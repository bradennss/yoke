use std::path::Path;

use anyhow::Result;

pub fn run(project_dir: &Path) -> Result<()> {
    let content = crate::knowledge::load(project_dir)?;

    if content.is_empty() {
        println!("No knowledge base found. It will be created during phase handoffs.");
        return Ok(());
    }

    if content.trim().is_empty() {
        println!("Knowledge base is empty. It will be populated during phase handoffs.");
        return Ok(());
    }

    println!("{content}");
    Ok(())
}
