use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::{Depth, IntentState, IntentStatus};

pub struct IntentStore {
    intents_dir: PathBuf,
}

impl IntentStore {
    pub fn new(project_dir: &Path) -> Self {
        Self {
            intents_dir: project_dir.join(".yoke/intents"),
        }
    }

    pub fn intents_dir(&self) -> &Path {
        &self.intents_dir
    }

    pub fn list(&self) -> Result<Vec<IntentState>> {
        if !self.intents_dir.exists() {
            return Ok(Vec::new());
        }

        let mut intents = Vec::new();

        let entries = std::fs::read_dir(&self.intents_dir)
            .with_context(|| format!("reading {}", self.intents_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let json_path = entry.path().join("intent.json");
            if json_path.exists() {
                let intent = IntentState::load(&json_path)?;
                intents.push(intent);
            }
        }

        intents.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(intents)
    }

    pub fn load(&self, id: &str) -> Result<IntentState> {
        if !self.intents_dir.exists() {
            bail!(
                "intents directory does not exist: {}",
                self.intents_dir.display()
            );
        }

        let prefix = format!("{id}-");

        let entries = std::fs::read_dir(&self.intents_dir)
            .with_context(|| format!("reading {}", self.intents_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(&prefix) {
                let json_path = entry.path().join("intent.json");
                return IntentState::load(&json_path);
            }
        }

        bail!("intent {id} not found")
    }

    pub fn create(&self, intent: &IntentState) -> Result<()> {
        let dir = self.intent_dir(intent);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating intent directory {}", dir.display()))?;

        let json_path = dir.join("intent.json");
        intent.save(&json_path)
    }

    pub fn next_number(&self) -> Result<u32> {
        if !self.intents_dir.exists() {
            return Ok(1);
        }

        let mut max_number: u32 = 0;

        let entries = std::fs::read_dir(&self.intents_dir)
            .with_context(|| format!("reading {}", self.intents_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(number) = parse_intent_number(&name_str) {
                max_number = max_number.max(number);
            }
        }

        Ok(max_number + 1)
    }

    pub fn intent_dir(&self, intent: &IntentState) -> PathBuf {
        self.intents_dir.join(intent.dir_name())
    }

    pub fn ensure_dirs(&self, intent: &IntentState) -> Result<()> {
        let base = self.intent_dir(intent);
        for subdir in ["phases", "research", "plans", "handoffs"] {
            let path = base.join(subdir);
            std::fs::create_dir_all(&path)
                .with_context(|| format!("creating {}", path.display()))?;
        }
        Ok(())
    }

    pub fn find_by_status(&self, status: IntentStatus) -> Result<Vec<IntentState>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|i| i.status == status)
            .collect())
    }

    pub fn active_full_depth(&self) -> Result<Option<IntentState>> {
        Ok(self.list()?.into_iter().find(|i| {
            i.depth == Depth::Full
                && (i.status == IntentStatus::InProgress || i.status == IntentStatus::Pending)
        }))
    }
}

fn parse_intent_number(dir_name: &str) -> Option<u32> {
    let rest = dir_name.strip_prefix("i-")?;
    let digits: &str = rest.split('-').next()?;
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{Classification, Depth};

    fn temp_store(name: &str) -> (IntentStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("yoke_store_test_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".yoke/intents")).unwrap();
        let store = IntentStore::new(&dir);
        (store, dir)
    }

    #[test]
    fn list_empty() {
        let (store, dir) = temp_store("list_empty");
        let intents = store.list().unwrap();
        assert!(intents.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_nonexistent_dir() {
        let store = IntentStore::new(Path::new("/tmp/yoke_store_nonexistent_12345"));
        let intents = store.list().unwrap();
        assert!(intents.is_empty());
    }

    #[test]
    fn create_and_load() {
        let (store, dir) = temp_store("create_and_load");

        let intent = IntentState::new(
            1,
            "Initial build".to_string(),
            "Build it".to_string(),
            Classification::Build,
            Depth::Full,
        );
        store.create(&intent).unwrap();

        let loaded = store.load("i-001").unwrap();
        assert_eq!(loaded.id, "i-001");
        assert_eq!(loaded.title, "Initial build");
        assert_eq!(loaded.classification, Classification::Build);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_not_found() {
        let (store, dir) = temp_store("load_not_found");
        let result = store.load("i-999");
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_multiple_sorted() {
        let (store, dir) = temp_store("list_multiple");

        let i3 = IntentState::new(
            3,
            "Third".to_string(),
            "Third intent".to_string(),
            Classification::Fix,
            Depth::Minimal,
        );
        let i1 = IntentState::new(
            1,
            "First".to_string(),
            "First intent".to_string(),
            Classification::Build,
            Depth::Full,
        );
        let i2 = IntentState::new(
            2,
            "Second".to_string(),
            "Second intent".to_string(),
            Classification::Feature,
            Depth::Light,
        );

        store.create(&i3).unwrap();
        store.create(&i1).unwrap();
        store.create(&i2).unwrap();

        let intents = store.list().unwrap();
        assert_eq!(intents.len(), 3);
        assert_eq!(intents[0].id, "i-001");
        assert_eq!(intents[1].id, "i-002");
        assert_eq!(intents[2].id, "i-003");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn next_number_empty() {
        let (store, dir) = temp_store("next_number_empty");
        assert_eq!(store.next_number().unwrap(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn next_number_with_existing() {
        let (store, dir) = temp_store("next_number_existing");

        let i1 = IntentState::new(
            1,
            "First".to_string(),
            "First".to_string(),
            Classification::Build,
            Depth::Full,
        );
        let i3 = IntentState::new(
            3,
            "Third".to_string(),
            "Third".to_string(),
            Classification::Fix,
            Depth::Minimal,
        );
        store.create(&i1).unwrap();
        store.create(&i3).unwrap();

        assert_eq!(store.next_number().unwrap(), 4);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_dirs_creates_subdirectories() {
        let (store, dir) = temp_store("ensure_dirs");

        let intent = IntentState::new(
            1,
            "Test".to_string(),
            "Test".to_string(),
            Classification::Build,
            Depth::Full,
        );
        store.create(&intent).unwrap();
        store.ensure_dirs(&intent).unwrap();

        let base = store.intent_dir(&intent);
        assert!(base.join("phases").is_dir());
        assert!(base.join("research").is_dir());
        assert!(base.join("plans").is_dir());
        assert!(base.join("handoffs").is_dir());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_by_status_filters() {
        let (store, dir) = temp_store("find_by_status");

        let mut pending = IntentState::new(
            1,
            "Pending".to_string(),
            "Pending".to_string(),
            Classification::Build,
            Depth::Full,
        );
        let mut in_progress = IntentState::new(
            2,
            "In progress".to_string(),
            "In progress".to_string(),
            Classification::Feature,
            Depth::Light,
        );
        in_progress.status = IntentStatus::InProgress;
        let mut completed = IntentState::new(
            3,
            "Completed".to_string(),
            "Done".to_string(),
            Classification::Fix,
            Depth::Minimal,
        );
        completed.status = IntentStatus::Completed;

        store.create(&pending).unwrap();
        store.create(&in_progress).unwrap();
        store.create(&completed).unwrap();

        // Suppress unused variable warning; pending is used for its default status.
        let _ = &mut pending;

        let found = store.find_by_status(IntentStatus::InProgress).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "i-002");

        let found = store.find_by_status(IntentStatus::Pending).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "i-001");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn active_full_depth_finds_in_progress() {
        let (store, dir) = temp_store("active_full");

        let mut full = IntentState::new(
            1,
            "Full build".to_string(),
            "Full".to_string(),
            Classification::Build,
            Depth::Full,
        );
        full.status = IntentStatus::InProgress;

        let mut light = IntentState::new(
            2,
            "Light work".to_string(),
            "Light".to_string(),
            Classification::Feature,
            Depth::Light,
        );
        light.status = IntentStatus::InProgress;

        store.create(&full).unwrap();
        store.create(&light).unwrap();

        let active = store.active_full_depth().unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, "i-001");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn active_full_depth_none_when_completed() {
        let (store, dir) = temp_store("active_full_completed");

        let mut full = IntentState::new(
            1,
            "Done build".to_string(),
            "Done".to_string(),
            Classification::Build,
            Depth::Full,
        );
        full.status = IntentStatus::Completed;
        store.create(&full).unwrap();

        let active = store.active_full_depth().unwrap();
        assert!(active.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_intent_number_valid() {
        assert_eq!(parse_intent_number("i-001-initial-build"), Some(1));
        assert_eq!(parse_intent_number("i-042-cleanup"), Some(42));
        assert_eq!(parse_intent_number("i-100-big-feature"), Some(100));
    }

    #[test]
    fn parse_intent_number_invalid() {
        assert_eq!(parse_intent_number("not-an-intent"), None);
        assert_eq!(parse_intent_number(""), None);
        assert_eq!(parse_intent_number("i-"), None);
        assert_eq!(parse_intent_number("i-abc-thing"), None);
    }

    #[test]
    fn intent_dir_path() {
        let store = IntentStore::new(Path::new("/project"));
        let intent = IntentState::new(
            7,
            "Auth flow".to_string(),
            "Add auth".to_string(),
            Classification::Feature,
            Depth::Light,
        );
        assert_eq!(
            store.intent_dir(&intent),
            PathBuf::from("/project/.yoke/intents/i-007-auth-flow")
        );
    }
}
