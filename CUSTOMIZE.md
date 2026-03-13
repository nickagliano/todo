# todo — Customization Guide

This is an [Extremely Personal Software (EPS)](https://epm.sh) harness. It is
**functional by default but deliberately incomplete** — the three ports below are
where you make it yours.

## Getting Started

```sh
git clone https://github.com/nickagliano/todo
cd todo
./run.sh add "My first task"
./run.sh list
./run.sh done 1
```

---

## Port 1: Storage (`src/storage.rs`)

**What it does:** Controls where tasks are persisted.

**Default:** `JsonStorage` — writes a JSON array to
`~/.config/todo/tasks.json`. Readable, but not great for concurrent
access, large lists, or syncing.

**How to customize:** Implement the `Storage` trait and swap it in `main.rs`:

```rust
pub trait Storage {
    fn load(&self) -> Result<Vec<Task>>;
    fn save(&self, tasks: &[Task]) -> Result<()>;
}
```

Example replacements:
- `SqliteStorage` — persist to a local SQLite database
- `HttpStorage` — POST/GET against a remote REST API
- `EncryptedStorage` — wrap any other storage with at-rest encryption
- `MultiStorage` — write to two backends simultaneously

Wire it up in `main.rs`:
```rust
// Before:
let storage = JsonStorage::default();
// After:
let storage = SqliteStorage::new("~/.config/todo/tasks.db")?;
```

---

## Port 2: Formatter (`src/formatter.rs`)

**What it does:** Controls how tasks are rendered to the terminal on `list`.

**Default:** `PlainText` — a plain numbered checklist:
```
[ ] #1: Buy groceries
[x] #2: Walk the dog
```

**How to customize:** Implement the `Formatter` trait and swap it in `main.rs`:

```rust
pub trait Formatter {
    fn format(&self, tasks: &[Task]) -> String;
}
```

Example replacements:
- `TableFormatter` — columnar output with done/pending counts
- `MarkdownFormatter` — `- [ ] task text` for pasting into docs
- `JsonFormatter` — machine-readable output for piping into other tools
- `PriorityFormatter` — group and color-code by a tag you add to `Task`

Wire it up in `main.rs`:
```rust
// Before:
let formatter = PlainText;
// After:
let formatter = TableFormatter::new();
```

---

## Port 3: Lifecycle Hooks (`hooks/`)

**What it does:** Runs shell scripts after add / complete / delete operations.

**Default:** Three stub scripts that do nothing. They're there so you can add
behavior without touching Rust.

**How to customize:** Edit or replace the scripts in `hooks/`. Each script
receives `$1 = task id` and `$2 = task text`.

| Script | Fires when |
|---|---|
| `hooks/on_add.sh` | A task is added |
| `hooks/on_complete.sh` | A task is marked done |
| `hooks/on_delete.sh` | A task is deleted |

Example uses:
- Send a macOS notification via `osascript`
- Log completions to a daily markdown file
- POST to a webhook (Slack, IFTTT, etc.)
- Sync state to another tool via its CLI

Example `hooks/on_complete.sh`:
```sh
#!/usr/bin/env bash
osascript -e "display notification \"Done: $2\" with title \"todo\""
```

---

---

## Port 4: Reading List (`src/reading.rs`)

**What it does:** Controls where reading list items are stored.

**Default:** `JsonReadingList` — writes a JSON array to
`~/.config/todo/reading.json`.

**How to customize:** Implement the `ReadingList` trait and swap it in `serve.rs`:

```rust
pub trait ReadingList {
    fn load(&self) -> Result<Vec<ReadingItem>>;
    fn save(&self, items: &[ReadingItem]) -> Result<()>;
}
```

Example swaps:
- `SqliteReadingList` — full-text search across saved articles
- `PocketSync` — synced to a remote API (Pocket, Raindrop)
- `BookmarksImport` — browser bookmarks file import/export

To swap: change `JsonReadingList::default()` in `serve.rs` to your implementation.

---

## What's deliberately missing

The following are ports, not bugs. Add them yourself:

- **Priorities / tags** — `Task` has `id`, `text`, `done`. Extend the struct.
- **Due dates** — not in the model. Add a `due: Option<String>` field.
- **Recurring tasks** — no scheduling. Wire it from a hook or cron job.
- **Multiple lists** — one flat list by default. Add a `list: String` field or
  a second storage key.
- **Undo** — no history. Implement it in storage or as a git-backed store.
