use next_rust::prelude::*;

use crate::store::{Todo, TODOS};

#[derive(serde::Deserialize)]
struct NewTodo {
    title: String,
}

pub async fn GET() -> Response {
    let todos = TODOS.lock().unwrap().clone();
    Response::json(&todos)
}

pub async fn POST(mut req: Request) -> Result<Response> {
    let input: NewTodo = req.json().await?;
    if input.title.trim().is_empty() {
        return Err(Error::validation([("title", "must not be empty")]));
    }
    let mut todos = TODOS.lock().unwrap();
    let todo = Todo { id: todos.iter().map(|t| t.id).max().unwrap_or(0) + 1, title: input.title, done: false };
    todos.push(todo.clone());
    Ok(Response::json(&todo).with_status(201))
}
