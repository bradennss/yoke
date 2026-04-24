use std::path::Path;

use anyhow::Result;

use crate::intent::store::IntentStore;

pub fn run(project_dir: &Path) -> Result<()> {
    let store = IntentStore::new(project_dir);
    let intents = store.list()?;

    if intents.is_empty() {
        println!("No intents found. Create one with `yoke new`.");
        return Ok(());
    }

    println!(
        "{:<8} {:<30} {:<12} {:<8} {:<12} {:>10}",
        "ID", "Title", "Class", "Depth", "Status", "Cost"
    );
    println!("{}", "-".repeat(84));

    for intent in &intents {
        let title: String = if intent.title.len() > 28 {
            let mut t: String = intent.title.chars().take(25).collect();
            t.push_str("...");
            t
        } else {
            intent.title.clone()
        };

        println!(
            "{:<8} {:<30} {:<12} {:<8} {:<12} ${:>9.4}",
            intent.id,
            title,
            intent.classification,
            intent.depth,
            intent.status,
            intent.total_cost_usd,
        );
    }

    Ok(())
}
