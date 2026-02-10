//! Error types for the todo API client.
//!
//! # Design
//! `NotFound` gets a dedicated variant because callers frequently distinguish
//! "the resource does not exist" from "the server returned an unexpected
//! status." All other non-2xx responses land in `HttpError` with the raw
//! status code and body for debugging.

use std::fmt;

/// Errors returned by `TodoClient` parse methods.
#[derive(Debug)]
pub enum ApiError {
    /// The server returned 404 — the requested todo does not exist.
    NotFound,

    /// The server returned a non-2xx status other than 404.
    HttpError { status: u16, body: String },

    /// The response body could not be deserialized into the expected type.
    DeserializationError(String),

    /// The request payload could not be serialized to JSON.
    SerializationError(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::NotFound => write!(f, "resource not found"),
            ApiError::HttpError { status, body } => {
                write!(f, "HTTP {status}: {body}")
            }
            ApiError::DeserializationError(msg) => {
                write!(f, "deserialization failed: {msg}")
            }
            ApiError::SerializationError(msg) => {
                write!(f, "serialization failed: {msg}")
            }
        }
    }
}

impl std::error::Error for ApiError {}
