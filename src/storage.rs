use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::task::Task;

// ─── PORT: Storage ────────────────────────────────────────────────────────────
//
// Swap `JsonStorage` for any type that implements this trait to change where
// tasks live — SQLite, a remote API, an in-memory store, a plain text file,
// whatever makes sense for your workflow.
//
// Implement the two methods and pass your type to `run()` in main.rs.

pub trait Storage {
    fn load(&self) -> Result<Vec<Task>>;
    fn save(&self, tasks: &[Task]) -> Result<()>;
}

// ─── Default: JSON file ───────────────────────────────────────────────────────
//
// Stores tasks as a JSON array at ~/.config/simple_todo/tasks.json.
// This is a starting point, not a recommendation.

pub struct JsonStorage {
    path: PathBuf,
}

impl JsonStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Default for JsonStorage {
    fn default() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("todo")
            .join("tasks.json");
        Self { path }
    }
}

impl Storage for JsonStorage {
    fn load(&self) -> Result<Vec<Task>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        serde_json::from_str(&data).context("failed to parse tasks.json")
    }

    fn save(&self, tasks: &[Task]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(tasks)?;
        std::fs::write(&self.path, data)
            .with_context(|| format!("failed to write {}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn storage_in(dir: &TempDir) -> JsonStorage {
        JsonStorage { path: dir.path().join("tasks.json") }
    }

    #[test]
    fn load_returns_empty_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let s = storage_in(&dir);
        let tasks = s.load().unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let s = storage_in(&dir);
        let tasks = vec![
            Task::new(1, "first".to_string()),
            Task { id: 2, text: "second".to_string(), done: true },
        ];
        s.save(&tasks).unwrap();
        let loaded = s.load().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].text, "first");
        assert!(!loaded[0].done);
        assert_eq!(loaded[1].text, "second");
        assert!(loaded[1].done);
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let s = JsonStorage { path: dir.path().join("a/b/c/tasks.json") };
        s.save(&[]).unwrap();
        assert!(s.path.exists());
    }

    #[test]
    fn save_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let s = storage_in(&dir);
        s.save(&[Task::new(1, "old".to_string())]).unwrap();
        s.save(&[Task::new(1, "new".to_string())]).unwrap();
        let loaded = s.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "new");
    }

    #[test]
    fn load_empty_vec_from_empty_file() {
        let dir = TempDir::new().unwrap();
        let s = storage_in(&dir);
        s.save(&[]).unwrap();
        let loaded = s.load().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn save_preserves_done_state() {
        let dir = TempDir::new().unwrap();
        let s = storage_in(&dir);
        let mut t = Task::new(1, "task".to_string());
        t.done = true;
        s.save(&[t]).unwrap();
        let loaded = s.load().unwrap();
        assert!(loaded[0].done);
    }
}
