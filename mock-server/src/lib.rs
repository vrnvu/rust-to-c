use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::RwLock};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
}

#[derive(Deserialize)]
pub struct CreateTodo {
    pub title: String,
    #[serde(default)]
    pub completed: bool,
}

#[derive(Deserialize)]
pub struct UpdateTodo {
    pub title: Option<String>,
    pub completed: Option<bool>,
}

pub type Db = Arc<RwLock<HashMap<Uuid, Todo>>>;

pub fn app() -> Router {
    let db: Db = Arc::new(RwLock::new(HashMap::new()));
    Router::new()
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/{id}", get(get_todo).put(update_todo).delete(delete_todo))
        .with_state(db)
}

pub async fn run(listener: TcpListener) -> Result<(), std::io::Error> {
    axum::serve(listener, app()).await
}

async fn list_todos(State(db): State<Db>) -> Json<Vec<Todo>> {
    let todos = db.read().await;
    Json(todos.values().cloned().collect())
}

async fn create_todo(
    State(db): State<Db>,
    Json(input): Json<CreateTodo>,
) -> (StatusCode, Json<Todo>) {
    let todo = Todo {
        id: Uuid::new_v4(),
        title: input.title,
        completed: input.completed,
    };
    db.write().await.insert(todo.id, todo.clone());
    (StatusCode::CREATED, Json(todo))
}

async fn get_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
) -> Result<Json<Todo>, StatusCode> {
    let todos = db.read().await;
    todos.get(&id).cloned().map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn update_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTodo>,
) -> Result<Json<Todo>, StatusCode> {
    let mut todos = db.write().await;
    let todo = todos.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    if let Some(title) = input.title {
        todo.title = title;
    }
    if let Some(completed) = input.completed {
        todo.completed = completed;
    }
    Ok(Json(todo.clone()))
}

async fn delete_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let mut todos = db.write().await;
    todos.remove(&id).map(|_| StatusCode::NO_CONTENT).ok_or(StatusCode::NOT_FOUND)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_serializes_to_json() {
        let todo = Todo {
            id: Uuid::nil(),
            title: "Test".to_string(),
            completed: false,
        };
        let json = serde_json::to_value(&todo).unwrap();
        assert_eq!(json["id"], "00000000-0000-0000-0000-000000000000");
        assert_eq!(json["title"], "Test");
        assert_eq!(json["completed"], false);
    }

    #[test]
    fn todo_roundtrips_through_json() {
        let todo = Todo {
            id: Uuid::new_v4(),
            title: "Roundtrip".to_string(),
            completed: true,
        };
        let json = serde_json::to_string(&todo).unwrap();
        let back: Todo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, todo.id);
        assert_eq!(back.title, todo.title);
        assert_eq!(back.completed, todo.completed);
    }

    #[test]
    fn create_todo_defaults_completed_to_false() {
        let input: CreateTodo = serde_json::from_str(r#"{"title":"No completed field"}"#).unwrap();
        assert_eq!(input.title, "No completed field");
        assert!(!input.completed);
    }

    #[test]
    fn create_todo_accepts_explicit_completed() {
        let input: CreateTodo =
            serde_json::from_str(r#"{"title":"Done","completed":true}"#).unwrap();
        assert!(input.completed);
    }

    #[test]
    fn create_todo_rejects_missing_title() {
        let result: Result<CreateTodo, _> = serde_json::from_str(r#"{"completed":true}"#);
        assert!(result.is_err());
    }

    #[test]
    fn update_todo_all_fields_optional() {
        let input: UpdateTodo = serde_json::from_str(r#"{}"#).unwrap();
        assert!(input.title.is_none());
        assert!(input.completed.is_none());
    }

    #[test]
    fn update_todo_partial_fields() {
        let input: UpdateTodo = serde_json::from_str(r#"{"title":"New title"}"#).unwrap();
        assert_eq!(input.title.as_deref(), Some("New title"));
        assert!(input.completed.is_none());
    }
}
