use crate::task::Task;

// ─── PORT: Formatter ──────────────────────────────────────────────────────────
//
// Swap `PlainText` for any type that implements this trait to change how tasks
// are rendered — rich tables, markdown, JSON, color-coded by priority, etc.
//
// Implement `format` and pass your type to `run()` in main.rs.

pub trait Formatter {
    fn format(&self, tasks: &[Task]) -> String;
}

// ─── Default: plain text list ─────────────────────────────────────────────────
//
// [ ] #1: Buy groceries
// [x] #2: Walk the dog

pub struct PlainText;

impl Formatter for PlainText {
    fn format(&self, tasks: &[Task]) -> String {
        if tasks.is_empty() {
            return "No tasks.".to_string();
        }
        tasks
            .iter()
            .map(|t| {
                let check = if t.done { "x" } else { " " };
                format!("[{check}] #{}: {}", t.id, t.text)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;

    #[test]
    fn empty_list_returns_no_tasks_message() {
        assert_eq!(PlainText.format(&[]), "No tasks.");
    }

    #[test]
    fn pending_task_shows_empty_checkbox() {
        let t = Task::new(1, "Buy milk".to_string());
        let out = PlainText.format(&[t]);
        assert_eq!(out, "[ ] #1: Buy milk");
    }

    #[test]
    fn done_task_shows_x_checkbox() {
        let t = Task { id: 3, text: "Walk dog".to_string(), done: true };
        let out = PlainText.format(&[t]);
        assert_eq!(out, "[x] #3: Walk dog");
    }

    #[test]
    fn multiple_tasks_joined_by_newline() {
        let tasks = vec![
            Task::new(1, "first".to_string()),
            Task { id: 2, text: "second".to_string(), done: true },
        ];
        let out = PlainText.format(&tasks);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "[ ] #1: first");
        assert_eq!(lines[1], "[x] #2: second");
    }

    #[test]
    fn all_done_list() {
        let tasks = vec![
            Task { id: 1, text: "a".to_string(), done: true },
            Task { id: 2, text: "b".to_string(), done: true },
        ];
        let out = PlainText.format(&tasks);
        assert!(out.lines().all(|l| l.starts_with("[x]")));
    }

    #[test]
    fn id_included_in_output() {
        let t = Task::new(42, "test".to_string());
        assert!(PlainText.format(&[t]).contains("#42"));
    }
}
