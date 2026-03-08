# todo

A minimal personal todo/house-projects/reading-list app served over HTTP and managed via EPC.

## Data storage

User data lives in the OS config directory — **not** in the repo:

| File | Path (macOS) |
|------|-------------|
| Tasks | `~/Library/Application Support/todo/tasks.json` |
| House projects | `~/Library/Application Support/todo/house_projects.json` |
| Reading list | `~/Library/Application Support/todo/reading.json` |

On Linux this would be `~/.config/todo/` instead. The `dirs` crate handles the difference.

Files are created automatically on first write. If you rename the app, the directory name changes too — migrate existing files manually or you'll start with empty data.

## Running

```sh
epc deploy todo   # first time
epc restart todo  # after rebuilding
```

Or directly:

```sh
./serve.sh
```
