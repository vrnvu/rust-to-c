//! Domain DTOs for the todo API.
//!
//! # Design
//! These types mirror the mock-server's schema but are defined independently.
//! The core crate will gain `#[repr(C)]` and FFI attributes in Phase 3;
//! keeping the types separate avoids coupling the FFI surface to Axum internals.
//! Integration tests catch any schema drift between the two crates.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single todo item returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
}

/// Request payload for creating a new todo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTodo {
    pub title: String,
    #[serde(default)]
    pub completed: bool,
}

/// Request payload for updating an existing todo. Only the fields present in
/// the JSON are applied; omitted fields remain unchanged on the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTodo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
}
