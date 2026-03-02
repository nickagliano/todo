use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u32,
    pub text: String,
    pub done: bool,
}

impl Task {
    pub fn new(id: u32, text: String) -> Self {
        Self { id, text, done: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_is_not_done() {
        let t = Task::new(1, "test".to_string());
        assert_eq!(t.id, 1);
        assert_eq!(t.text, "test");
        assert!(!t.done);
    }

    #[test]
    fn serialize_roundtrip() {
        let t = Task { id: 7, text: "hello".to_string(), done: true };
        let json = serde_json::to_string(&t).unwrap();
        let back: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, t.id);
        assert_eq!(back.text, t.text);
        assert_eq!(back.done, t.done);
    }

    #[test]
    fn vec_of_tasks_roundtrip() {
        let tasks = vec![
            Task::new(1, "a".to_string()),
            Task { id: 2, text: "b".to_string(), done: true },
        ];
        let json = serde_json::to_string(&tasks).unwrap();
        let back: Vec<Task> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
        assert!(!back[0].done);
        assert!(back[1].done);
    }
}
