use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub done: bool,
}

pub static TODOS: LazyLock<Mutex<Vec<Todo>>> =
    LazyLock::new(|| Mutex::new(vec![Todo { id: 1, title: "Try Next Rust".into(), done: false }]));
