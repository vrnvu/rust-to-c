//! Stateless HTTP request builder and response parser for the todo API.
//!
//! # Design
//! `TodoClient` holds only a `base_url` and carries no mutable state between
//! calls. Each CRUD operation is split into a `build_*` method that produces
//! an `HttpRequest` and a `parse_*` method that consumes an `HttpResponse`.
//! The caller executes the actual HTTP round-trip, keeping the core
//! deterministic and free of I/O dependencies.

use uuid::Uuid;

use crate::error::ApiError;
use crate::http::{HttpMethod, HttpRequest, HttpResponse};
use crate::types::{CreateTodo, Todo, UpdateTodo};

/// Synchronous, stateless client for the todo API.
///
/// Builds `HttpRequest` values and parses `HttpResponse` values without
/// touching the network. The caller is responsible for executing the HTTP
/// round-trip between `build_*` and `parse_*`.
#[derive(Debug, Clone)]
pub struct TodoClient {
    base_url: String,
}

impl TodoClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn build_list_todos(&self) -> HttpRequest {
        HttpRequest {
            method: HttpMethod::Get,
            path: format!("{}/todos", self.base_url),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn build_get_todo(&self, id: Uuid) -> HttpRequest {
        HttpRequest {
            method: HttpMethod::Get,
            path: format!("{}/todos/{id}", self.base_url),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn build_create_todo(&self, input: &CreateTodo) -> Result<HttpRequest, ApiError> {
        let body = serde_json::to_string(input).map_err(|e| ApiError::SerializationError(e.to_string()))?;
        Ok(HttpRequest {
            method: HttpMethod::Post,
            path: format!("{}/todos", self.base_url),
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: Some(body),
        })
    }

    pub fn build_update_todo(&self, id: Uuid, input: &UpdateTodo) -> Result<HttpRequest, ApiError> {
        let body = serde_json::to_string(input).map_err(|e| ApiError::SerializationError(e.to_string()))?;
        Ok(HttpRequest {
            method: HttpMethod::Put,
            path: format!("{}/todos/{id}", self.base_url),
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: Some(body),
        })
    }

    pub fn build_delete_todo(&self, id: Uuid) -> HttpRequest {
        HttpRequest {
            method: HttpMethod::Delete,
            path: format!("{}/todos/{id}", self.base_url),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn parse_list_todos(&self, response: HttpResponse) -> Result<Vec<Todo>, ApiError> {
        check_status(&response, 200)?;
        serde_json::from_str(&response.body).map_err(|e| ApiError::DeserializationError(e.to_string()))
    }

    pub fn parse_get_todo(&self, response: HttpResponse) -> Result<Todo, ApiError> {
        check_status(&response, 200)?;
        serde_json::from_str(&response.body).map_err(|e| ApiError::DeserializationError(e.to_string()))
    }

    pub fn parse_create_todo(&self, response: HttpResponse) -> Result<Todo, ApiError> {
        check_status(&response, 201)?;
        serde_json::from_str(&response.body).map_err(|e| ApiError::DeserializationError(e.to_string()))
    }

    pub fn parse_update_todo(&self, response: HttpResponse) -> Result<Todo, ApiError> {
        check_status(&response, 200)?;
        serde_json::from_str(&response.body).map_err(|e| ApiError::DeserializationError(e.to_string()))
    }

    pub fn parse_delete_todo(&self, response: HttpResponse) -> Result<(), ApiError> {
        check_status(&response, 204)?;
        Ok(())
    }
}

/// Map non-success status codes to the appropriate `ApiError` variant.
fn check_status(response: &HttpResponse, expected: u16) -> Result<(), ApiError> {
    if response.status == expected {
        return Ok(());
    }
    if response.status == 404 {
        return Err(ApiError::NotFound);
    }
    Err(ApiError::HttpError {
        status: response.status,
        body: response.body.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> TodoClient {
        TodoClient::new("http://localhost:3000")
    }

    #[test]
    fn build_list_todos_produces_correct_request() {
        let req = client().build_list_todos();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.path, "http://localhost:3000/todos");
        assert!(req.body.is_none());
        assert!(req.headers.is_empty());
    }

    #[test]
    fn build_get_todo_produces_correct_request() {
        let id = Uuid::nil();
        let req = client().build_get_todo(id);
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(
            req.path,
            "http://localhost:3000/todos/00000000-0000-0000-0000-000000000000"
        );
        assert!(req.body.is_none());
    }

    #[test]
    fn build_create_todo_produces_correct_request() {
        let input = CreateTodo {
            title: "Buy milk".to_string(),
            completed: false,
        };
        let req = client().build_create_todo(&input).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "http://localhost:3000/todos");
        assert_eq!(
            req.headers,
            vec![("content-type".to_string(), "application/json".to_string())]
        );
        let body: serde_json::Value = serde_json::from_str(req.body.as_deref().unwrap()).unwrap();
        assert_eq!(body["title"], "Buy milk");
        assert_eq!(body["completed"], false);
    }

    #[test]
    fn build_update_todo_produces_correct_request() {
        let id = Uuid::nil();
        let input = UpdateTodo {
            title: Some("Updated".to_string()),
            completed: None,
        };
        let req = client().build_update_todo(id, &input).unwrap();
        assert_eq!(req.method, HttpMethod::Put);
        let body: serde_json::Value = serde_json::from_str(req.body.as_deref().unwrap()).unwrap();
        assert_eq!(body["title"], "Updated");
        assert!(body.get("completed").is_none());
    }

    #[test]
    fn build_delete_todo_produces_correct_request() {
        let id = Uuid::nil();
        let req = client().build_delete_todo(id);
        assert_eq!(req.method, HttpMethod::Delete);
        assert!(req.body.is_none());
    }

    #[test]
    fn parse_list_todos_success() {
        let response = HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: r#"[{"id":"00000000-0000-0000-0000-000000000001","title":"Test","completed":false}]"#.to_string(),
        };
        let todos = client().parse_list_todos(response).unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "Test");
    }

    #[test]
    fn parse_get_todo_not_found() {
        let response = HttpResponse {
            status: 404,
            headers: Vec::new(),
            body: String::new(),
        };
        let err = client().parse_get_todo(response).unwrap_err();
        assert!(matches!(err, ApiError::NotFound));
    }

    #[test]
    fn parse_create_todo_success() {
        let response = HttpResponse {
            status: 201,
            headers: Vec::new(),
            body: r#"{"id":"00000000-0000-0000-0000-000000000001","title":"New","completed":false}"#.to_string(),
        };
        let todo = client().parse_create_todo(response).unwrap();
        assert_eq!(todo.title, "New");
    }

    #[test]
    fn parse_create_todo_wrong_status() {
        let response = HttpResponse {
            status: 500,
            headers: Vec::new(),
            body: "internal error".to_string(),
        };
        let err = client().parse_create_todo(response).unwrap_err();
        assert!(matches!(err, ApiError::HttpError { status: 500, .. }));
    }

    #[test]
    fn parse_update_todo_success() {
        let response = HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: r#"{"id":"00000000-0000-0000-0000-000000000001","title":"Updated","completed":true}"#.to_string(),
        };
        let todo = client().parse_update_todo(response).unwrap();
        assert_eq!(todo.title, "Updated");
        assert!(todo.completed);
    }

    #[test]
    fn parse_delete_todo_success() {
        let response = HttpResponse {
            status: 204,
            headers: Vec::new(),
            body: String::new(),
        };
        assert!(client().parse_delete_todo(response).is_ok());
    }

    #[test]
    fn parse_delete_todo_not_found() {
        let response = HttpResponse {
            status: 404,
            headers: Vec::new(),
            body: String::new(),
        };
        let err = client().parse_delete_todo(response).unwrap_err();
        assert!(matches!(err, ApiError::NotFound));
    }

    #[test]
    fn trailing_slash_is_stripped() {
        let client = TodoClient::new("http://localhost:3000/");
        let req = client.build_list_todos();
        assert_eq!(req.path, "http://localhost:3000/todos");
    }

    #[test]
    fn parse_list_todos_bad_json() {
        let response = HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: "not json".to_string(),
        };
        let err = client().parse_list_todos(response).unwrap_err();
        assert!(matches!(err, ApiError::DeserializationError(_)));
    }
}
