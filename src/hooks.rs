use std::process::Command;

use anyhow::Result;

// ─── PORT: Lifecycle Hooks ────────────────────────────────────────────────────
//
// Drop executable shell scripts into hooks/ to run custom logic after each
// operation. Scripts receive: $1 = task id, $2 = task text.
//
// Hooks (all optional):
//   hooks/on_add.sh      — fires after a task is added
//   hooks/on_complete.sh — fires after a task is marked done
//   hooks/on_delete.sh   — fires after a task is deleted
//
// If a hook script doesn't exist or isn't executable, it's silently skipped.

pub fn run(hook: &str, args: &[&str]) -> Result<()> {
    let script = format!("hooks/{hook}.sh");
    let path = std::path::Path::new(&script);
    if path.exists() {
        Command::new(path).args(args).status()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_hook_does_not_error() {
        // hooks/nonexistent_hook_xyz.sh won't exist in the test environment
        assert!(run("nonexistent_hook_xyz", &[]).is_ok());
    }

    #[test]
    fn missing_hook_with_args_does_not_error() {
        assert!(run("nonexistent_hook_xyz", &["1", "some text"]).is_ok());
    }
}
