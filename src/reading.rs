use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ─── PORT: ReadingList ────────────────────────────────────────────────────────
//
// Swap `JsonReadingList` for any type that implements this trait to change where
// reading list items live — SQLite, a remote bookmarks API, a browser export,
// whatever makes sense for your workflow.

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReadingItem {
    pub id: u32,
    pub url: String,
    pub title: String,
    pub read: bool,
}

pub trait ReadingList {
    fn load(&self) -> Result<Vec<ReadingItem>>;
    fn save(&self, items: &[ReadingItem]) -> Result<()>;
}

// ─── Default: JSON file ───────────────────────────────────────────────────────
//
// Stores items as a JSON array at ~/.config/simple_todo/reading.json.

pub struct JsonReadingList {
    pub path: PathBuf,
}

impl Default for JsonReadingList {
    fn default() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("todo")
            .join("reading.json");
        Self { path }
    }
}

impl ReadingList for JsonReadingList {
    fn load(&self) -> Result<Vec<ReadingItem>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        serde_json::from_str(&data).context("failed to parse reading.json")
    }

    fn save(&self, items: &[ReadingItem]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(items)?;
        std::fs::write(&self.path, data)
            .with_context(|| format!("failed to write {}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn reading_list_in(dir: &TempDir) -> JsonReadingList {
        JsonReadingList { path: dir.path().join("reading.json") }
    }

    #[test]
    fn load_returns_empty_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let rl = reading_list_in(&dir);
        let items = rl.load().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let rl = reading_list_in(&dir);
        let items = vec![
            ReadingItem { id: 1, url: "https://example.com".into(), title: "Example".into(), read: false },
            ReadingItem { id: 2, url: "https://rust-lang.org".into(), title: "Rust".into(), read: true },
        ];
        rl.save(&items).unwrap();
        let loaded = rl.load().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].url, "https://example.com");
        assert!(!loaded[0].read);
        assert_eq!(loaded[1].title, "Rust");
        assert!(loaded[1].read);
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let rl = JsonReadingList { path: dir.path().join("a/b/c/reading.json") };
        rl.save(&[]).unwrap();
        assert!(rl.path.exists());
    }

    #[test]
    fn default_resolves_to_config_dir() {
        let rl = JsonReadingList::default();
        assert!(rl.path.to_string_lossy().contains("todo"));
        assert!(rl.path.to_string_lossy().ends_with("reading.json"));
    }

    #[test]
    fn save_preserves_all_fields() {
        let dir = TempDir::new().unwrap();
        let rl = reading_list_in(&dir);
        let item = ReadingItem {
            id: 42,
            url: "https://doc.rust-lang.org".into(),
            title: "Rust Docs".into(),
            read: true,
        };
        rl.save(&[item.clone()]).unwrap();
        let loaded = rl.load().unwrap();
        assert_eq!(loaded[0], item);
    }

    #[test]
    fn save_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let rl = reading_list_in(&dir);
        let v1 = vec![ReadingItem { id: 1, url: "https://a.com".into(), title: "A".into(), read: false }];
        let v2 = vec![ReadingItem { id: 1, url: "https://b.com".into(), title: "B".into(), read: true }];
        rl.save(&v1).unwrap();
        rl.save(&v2).unwrap();
        let loaded = rl.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].url, "https://b.com");
    }
}
