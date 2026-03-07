/// Integration tests for the simple_todo CLI.
///
/// Each test gets an isolated HOME via TempDir so tasks never bleed between runs.
/// On macOS, `dirs::config_dir()` resolves to `$HOME/Library/Application Support`,
/// so overriding HOME is sufficient to isolate storage.
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[allow(deprecated)]
fn cmd(home: &TempDir) -> Command {
    let mut c = Command::cargo_bin("todo").unwrap();
    c.env("HOME", home.path());
    c
}

// ─── list ─────────────────────────────────────────────────────────────────────

#[test]
fn list_with_no_tasks_prints_no_tasks() {
    let home = TempDir::new().unwrap();
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks."));
}

// ─── add ─────────────────────────────────────────────────────────────────────

#[test]
fn add_prints_confirmation() {
    let home = TempDir::new().unwrap();
    cmd(&home)
        .args(["add", "Buy groceries"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added #1: Buy groceries"));
}

#[test]
fn add_then_list_shows_task() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Walk the dog"]).assert().success();
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Walk the dog"))
        .stdout(predicate::str::contains("[ ]"));
}

#[test]
fn add_multiple_increments_ids() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "First"]).assert().success()
        .stdout(predicate::str::contains("#1"));
    cmd(&home).args(["add", "Second"]).assert().success()
        .stdout(predicate::str::contains("#2"));
    cmd(&home).args(["add", "Third"]).assert().success()
        .stdout(predicate::str::contains("#3"));
}

#[test]
fn add_persists_across_invocations() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Persistent task"]).assert().success();
    // Second process, same HOME — must still see the task
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Persistent task"));
}

#[test]
fn add_multiple_all_appear_in_list() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Alpha"]).assert().success();
    cmd(&home).args(["add", "Beta"]).assert().success();
    cmd(&home).args(["add", "Gamma"]).assert().success();
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alpha"))
        .stdout(predicate::str::contains("Beta"))
        .stdout(predicate::str::contains("Gamma"));
}

// ─── done ─────────────────────────────────────────────────────────────────────

#[test]
fn done_prints_confirmation() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Finish report"]).assert().success();
    cmd(&home)
        .args(["done", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Done #1: Finish report"));
}

#[test]
fn done_marks_task_with_x_in_list() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Read book"]).assert().success();
    cmd(&home).args(["done", "1"]).assert().success();
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[x] #1: Read book"));
}

#[test]
fn done_on_nonexistent_id_prints_to_stderr() {
    let home = TempDir::new().unwrap();
    cmd(&home)
        .args(["done", "99"])
        .assert()
        .stderr(predicate::str::contains("No task #99"));
}

#[test]
fn done_only_marks_target_task() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Task A"]).assert().success();
    cmd(&home).args(["add", "Task B"]).assert().success();
    cmd(&home).args(["done", "1"]).assert().success();
    let out = cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("[x] #1: Task A"));
    assert!(out.contains("[ ] #2: Task B"));
}

// ─── delete ───────────────────────────────────────────────────────────────────

#[test]
fn delete_prints_confirmation() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Temp task"]).assert().success();
    cmd(&home)
        .args(["delete", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted #1: Temp task"));
}

#[test]
fn delete_removes_task_from_list() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Keep"]).assert().success();
    cmd(&home).args(["add", "Remove"]).assert().success();
    cmd(&home).args(["delete", "2"]).assert().success();
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Keep"))
        .stdout(predicate::str::contains("Remove").not());
}

#[test]
fn delete_on_nonexistent_id_prints_to_stderr() {
    let home = TempDir::new().unwrap();
    cmd(&home)
        .args(["delete", "42"])
        .assert()
        .stderr(predicate::str::contains("No task #42"));
}

#[test]
fn delete_all_tasks_shows_empty_list() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Only task"]).assert().success();
    cmd(&home).args(["delete", "1"]).assert().success();
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks."));
}

// ─── edge cases ───────────────────────────────────────────────────────────────

#[test]
fn done_task_can_be_deleted() {
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "Done then gone"]).assert().success();
    cmd(&home).args(["done", "1"]).assert().success();
    cmd(&home).args(["delete", "1"]).assert().success();
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks."));
}

#[test]
fn ids_are_stable_after_delete() {
    // Deleting task #1 does not renumber task #2 to #1
    let home = TempDir::new().unwrap();
    cmd(&home).args(["add", "First"]).assert().success();
    cmd(&home).args(["add", "Second"]).assert().success();
    cmd(&home).args(["delete", "1"]).assert().success();
    cmd(&home)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#2: Second"));
}
