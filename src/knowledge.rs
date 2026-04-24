use std::path::Path;

use anyhow::Result;

pub fn load(project_dir: &Path) -> Result<String> {
    let path = project_dir.join(".yoke/knowledge.md");
    if !path.exists() {
        return Ok(String::new());
    }
    Ok(std::fs::read_to_string(&path)?)
}

pub fn needs_compaction(project_dir: &Path, interval: usize) -> bool {
    let path = project_dir.join(".yoke/knowledge.md");
    if !path.exists() {
        return false;
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content.matches("[i-").count() >= interval
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_missing_file_returns_empty() {
        let dir = std::env::temp_dir().join("yoke_knowledge_missing_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = load(&dir).unwrap();
        assert!(result.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_empty_file_returns_empty() {
        let dir = std::env::temp_dir().join("yoke_knowledge_empty_test");
        let _ = fs::remove_dir_all(&dir);
        let yoke_dir = dir.join(".yoke");
        fs::create_dir_all(&yoke_dir).unwrap();
        fs::write(yoke_dir.join("knowledge.md"), "").unwrap();

        let result = load(&dir).unwrap();
        assert!(result.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_existing_file_returns_content() {
        let dir = std::env::temp_dir().join("yoke_knowledge_existing_test");
        let _ = fs::remove_dir_all(&dir);
        let yoke_dir = dir.join(".yoke");
        fs::create_dir_all(&yoke_dir).unwrap();
        let content = "# Knowledge\n\n[i-001] Some notes\n";
        fs::write(yoke_dir.join("knowledge.md"), content).unwrap();

        let result = load(&dir).unwrap();
        assert_eq!(result, content);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn needs_compaction_false_when_missing() {
        let dir = std::env::temp_dir().join("yoke_knowledge_compact_missing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        assert!(!needs_compaction(&dir, 5));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn needs_compaction_false_below_interval() {
        let dir = std::env::temp_dir().join("yoke_knowledge_compact_below");
        let _ = fs::remove_dir_all(&dir);
        let yoke_dir = dir.join(".yoke");
        fs::create_dir_all(&yoke_dir).unwrap();
        fs::write(yoke_dir.join("knowledge.md"), "[i-001] one\n[i-002] two\n").unwrap();

        assert!(!needs_compaction(&dir, 5));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn needs_compaction_true_at_interval() {
        let dir = std::env::temp_dir().join("yoke_knowledge_compact_at");
        let _ = fs::remove_dir_all(&dir);
        let yoke_dir = dir.join(".yoke");
        fs::create_dir_all(&yoke_dir).unwrap();
        let mut content = String::new();
        for i in 1..=5 {
            content.push_str(&format!("[i-{i:03}] entry {i}\n"));
        }
        fs::write(yoke_dir.join("knowledge.md"), &content).unwrap();

        assert!(needs_compaction(&dir, 5));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn needs_compaction_true_above_interval() {
        let dir = std::env::temp_dir().join("yoke_knowledge_compact_above");
        let _ = fs::remove_dir_all(&dir);
        let yoke_dir = dir.join(".yoke");
        fs::create_dir_all(&yoke_dir).unwrap();
        let mut content = String::new();
        for i in 1..=10 {
            content.push_str(&format!("[i-{i:03}] entry {i}\n"));
        }
        fs::write(yoke_dir.join("knowledge.md"), &content).unwrap();

        assert!(needs_compaction(&dir, 5));

        let _ = fs::remove_dir_all(&dir);
    }
}
