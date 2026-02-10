//! Synchronous API client core for the todo service.
//!
//! # Overview
//! Builds `HttpRequest` values and parses `HttpResponse` values without
//! touching the network (host-does-IO pattern). The caller executes the
//! actual HTTP round-trip, making the core fully deterministic and testable.
//!
//! # Design
//! - `TodoClient` is stateless — it holds only `base_url`.
//! - Each CRUD operation is split into `build_*` (produces request) and
//!   `parse_*` (consumes response), so the I/O boundary is explicit.
//! - Types use owned `String` / `Vec` fields to simplify future FFI mapping.
//! - DTOs are defined independently from the mock-server crate; integration
//!   tests catch schema drift.

pub mod client;
pub mod error;
pub mod http;
pub mod types;

pub use client::TodoClient;
pub use error::ApiError;
pub use http::{HttpMethod, HttpRequest, HttpResponse};
pub use types::{CreateTodo, Todo, UpdateTodo};
