use next_rust::prelude::*;

use crate::store::TODOS;

fn id(req: &Request) -> Result<u64> {
    req.param("id").and_then(|v| v.parse().ok()).ok_or_else(|| Error::http(400, "invalid id"))
}

pub async fn GET(req: Request) -> Result<Response> {
    let id = id(&req)?;
    let todos = TODOS.lock().unwrap();
    let todo = todos.iter().find(|t| t.id == id).or_not_found()?;
    Ok(Response::json(todo))
}

#[derive(serde::Deserialize)]
struct Patch {
    done: Option<bool>,
    title: Option<String>,
}

pub async fn PATCH(mut req: Request) -> Result<Response> {
    let id = id(&req)?;
    let patch: Patch = req.json().await?;
    let mut todos = TODOS.lock().unwrap();
    let todo = todos.iter_mut().find(|t| t.id == id).or_not_found()?;
    if let Some(done) = patch.done {
        todo.done = done;
    }
    if let Some(title) = patch.title {
        todo.title = title;
    }
    Ok(Response::json(todo))
}

pub async fn DELETE(req: Request) -> Result<Response> {
    let id = id(&req)?;
    TODOS.lock().unwrap().retain(|t| t.id != id);
    Ok(Response::status(204))
}
