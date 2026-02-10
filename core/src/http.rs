//! HTTP transport types for the host-does-IO pattern.
//!
//! # Design
//! These types describe HTTP requests and responses as plain data. The core
//! crate builds `HttpRequest` values and parses `HttpResponse` values without
//! ever touching the network — the caller (host) is responsible for executing
//! the actual I/O. This separation keeps the core deterministic and easy to
//! test, and maps cleanly to a C FFI boundary in later phases.
//!
//! All fields use owned types (`String`, `Vec`) so values can cross FFI
//! boundaries without lifetime concerns.

/// HTTP method for a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

/// An HTTP request described as plain data.
///
/// Built by `TodoClient::build_*` methods. The caller is responsible for
/// executing this request against the network and returning the corresponding
/// `HttpResponse`.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// An HTTP response described as plain data.
///
/// Constructed by the caller after executing an `HttpRequest`, then passed
/// to `TodoClient::parse_*` methods for deserialization.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}
