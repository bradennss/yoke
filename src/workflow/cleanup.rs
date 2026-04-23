use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// RAII guard that snapshots files in a directory before a step runs.
/// On drop (unless defused), deletes any files that were added during the step.
pub struct FileCleanupGuard {
    tracked_files: HashSet<PathBuf>,
    directory: PathBuf,
    defused: bool,
}

impl FileCleanupGuard {
    pub fn new(directory: &Path) -> Self {
        let tracked_files = snapshot_files(directory).unwrap_or_default();
        Self {
            tracked_files,
            directory: directory.to_path_buf(),
            defused: false,
        }
    }

    pub fn defuse(&mut self) {
        self.defused = true;
    }
}

impl Drop for FileCleanupGuard {
    fn drop(&mut self) {
        if self.defused {
            return;
        }
        let current_files = match snapshot_files(&self.directory) {
            Ok(files) => files,
            Err(_) => return,
        };
        for file in current_files.difference(&self.tracked_files) {
            let _ = std::fs::remove_file(file);
        }
    }
}

/// RAII guard that tracks whether a specific file existed before a step runs.
/// On drop (unless defused), removes the file if it was newly created.
pub struct SingleFileGuard {
    path: PathBuf,
    existed_before: bool,
    defused: bool,
}

impl SingleFileGuard {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            existed_before: path.exists(),
            defused: false,
        }
    }

    pub fn defuse(&mut self) {
        self.defused = true;
    }
}

impl Drop for SingleFileGuard {
    fn drop(&mut self) {
        if self.defused || self.existed_before {
            return;
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

fn snapshot_files(dir: &Path) -> std::io::Result<HashSet<PathBuf>> {
    let mut files = HashSet::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.insert(entry.path());
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_cleanup_guard_removes_new_files_on_drop() {
        let dir = std::env::temp_dir().join("yoke_cleanup_test_remove");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("existing.md"), "keep").unwrap();

        {
            let _guard = FileCleanupGuard::new(&dir);
            std::fs::write(dir.join("orphan.md"), "delete me").unwrap();
        }

        assert!(dir.join("existing.md").exists());
        assert!(!dir.join("orphan.md").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_cleanup_guard_keeps_files_when_defused() {
        let dir = std::env::temp_dir().join("yoke_cleanup_test_defuse");
        let _ = std::fs::create_dir_all(&dir);

        {
            let mut guard = FileCleanupGuard::new(&dir);
            std::fs::write(dir.join("new.md"), "keep").unwrap();
            guard.defuse();
        }

        assert!(dir.join("new.md").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_cleanup_guard_handles_nonexistent_directory() {
        let dir = Path::new("/tmp/yoke_cleanup_nonexistent_99999");
        let _guard = FileCleanupGuard::new(dir);
    }

    #[test]
    fn single_file_guard_removes_newly_created_file() {
        let dir = std::env::temp_dir().join("yoke_single_guard_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("new_file.md");

        {
            let _guard = SingleFileGuard::new(&path);
            std::fs::write(&path, "temporary").unwrap();
        }

        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_file_guard_keeps_preexisting_file() {
        let dir = std::env::temp_dir().join("yoke_single_guard_existing");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("existing.md");
        std::fs::write(&path, "original").unwrap();

        {
            let _guard = SingleFileGuard::new(&path);
            std::fs::write(&path, "modified").unwrap();
        }

        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "modified");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_file_guard_keeps_file_when_defused() {
        let dir = std::env::temp_dir().join("yoke_single_guard_defuse");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("new_kept.md");

        {
            let mut guard = SingleFileGuard::new(&path);
            std::fs::write(&path, "keep me").unwrap();
            guard.defuse();
        }

        assert!(path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_file_guard_noop_when_file_never_created() {
        let path = std::env::temp_dir().join("yoke_single_guard_noop.md");
        let _ = std::fs::remove_file(&path);

        {
            let _guard = SingleFileGuard::new(&path);
        }

        assert!(!path.exists());
    }
}
