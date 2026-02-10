//! C-ABI wrapper around `todo-core`.
//!
//! # Overview
//! Exposes the full todo CRUD API through `extern "C"` functions so any
//! language with a C FFI can build and parse HTTP requests/responses without
//! linking to Rust's async runtime or serde directly.
//!
//! # Design
//! - Every `extern "C"` function wraps its body in `catch_unwind` so panics
//!   never cross the FFI boundary.
//! - Per-operation `build_*` / `parse_*` mirrors the core API 1:1.
//! - A single `FfiTodoResult` envelope with `FfiDataTag` + `void* data`
//!   conveys success payloads and errors uniformly.
//! - The C caller owns all returned pointers and must call the matching
//!   `todo_free_*` function to release them.

pub mod types;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::catch_unwind;

use todo_core::http::HttpResponse;
use todo_core::types::{CreateTodo, UpdateTodo};

use types::*;

// ---------------------------------------------------------------------------
// Client lifecycle
// ---------------------------------------------------------------------------

/// Create a new `TodoClient` bound to `base_url`.
///
/// Returns null if `base_url` is null or if an internal panic occurs.
/// The caller must free the returned pointer with `todo_client_free`.
#[unsafe(no_mangle)]
pub extern "C" fn todo_client_new(base_url: *const c_char) -> *mut FfiTodoClient {
    catch_unwind(|| {
        if base_url.is_null() {
            return std::ptr::null_mut();
        }
        let url = unsafe { CStr::from_ptr(base_url) }.to_str().unwrap_or("");
        let client = todo_core::TodoClient::new(url);
        Box::into_raw(Box::new(FfiTodoClient { inner: client }))
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Free a `TodoClient` created by `todo_client_new`. Safe to call with null.
#[unsafe(no_mangle)]
pub extern "C" fn todo_client_free(client: *mut FfiTodoClient) {
    if !client.is_null() {
        let _ = catch_unwind(|| {
            drop(unsafe { Box::from_raw(client) });
        });
    }
}

// ---------------------------------------------------------------------------
// Build request functions
// ---------------------------------------------------------------------------

/// Build an HTTP request for listing all todos.
///
/// Returns null if `client` is null.
/// The caller must free the returned pointer with `todo_free_request`.
#[unsafe(no_mangle)]
pub extern "C" fn todo_build_list_todos(client: *const FfiTodoClient) -> *mut FfiHttpRequest {
    catch_unwind(|| {
        if client.is_null() {
            return std::ptr::null_mut();
        }
        let client = unsafe { &*client };
        let req = client.inner.build_list_todos();
        FfiHttpRequest::from_core(req)
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Build an HTTP request for fetching a single todo by id.
///
/// Returns null if `client` or `id` is null, or if `id` is not a valid UUID.
#[unsafe(no_mangle)]
pub extern "C" fn todo_build_get_todo(
    client: *const FfiTodoClient,
    id: *const c_char,
) -> *mut FfiHttpRequest {
    catch_unwind(|| {
        if client.is_null() || id.is_null() {
            return std::ptr::null_mut();
        }
        let client = unsafe { &*client };
        let id_str = unsafe { CStr::from_ptr(id) }.to_str().unwrap_or("");
        let uuid = match uuid::Uuid::parse_str(id_str) {
            Ok(u) => u,
            Err(_) => return std::ptr::null_mut(),
        };
        let req = client.inner.build_get_todo(uuid);
        FfiHttpRequest::from_core(req)
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Build an HTTP request for creating a new todo.
///
/// Returns null if `client` or `title` is null, or if serialization fails.
#[unsafe(no_mangle)]
pub extern "C" fn todo_build_create_todo(
    client: *const FfiTodoClient,
    title: *const c_char,
    completed: bool,
) -> *mut FfiHttpRequest {
    catch_unwind(|| {
        if client.is_null() || title.is_null() {
            return std::ptr::null_mut();
        }
        let client = unsafe { &*client };
        let title_str = unsafe { CStr::from_ptr(title) }
            .to_str()
            .unwrap_or("")
            .to_string();
        let input = CreateTodo {
            title: title_str,
            completed,
        };
        match client.inner.build_create_todo(&input) {
            Ok(req) => FfiHttpRequest::from_core(req),
            Err(_) => std::ptr::null_mut(),
        }
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Build an HTTP request for updating an existing todo.
///
/// `title` may be null (skip update). `completed` uses tri-state:
/// -1 = skip, 0 = false, 1 = true.
/// Returns null if `client` or `id` is null, or if `id` is not a valid UUID.
#[unsafe(no_mangle)]
pub extern "C" fn todo_build_update_todo(
    client: *const FfiTodoClient,
    id: *const c_char,
    title: *const c_char,
    completed: i32,
) -> *mut FfiHttpRequest {
    catch_unwind(|| {
        if client.is_null() || id.is_null() {
            return std::ptr::null_mut();
        }
        let client = unsafe { &*client };
        let id_str = unsafe { CStr::from_ptr(id) }.to_str().unwrap_or("");
        let uuid = match uuid::Uuid::parse_str(id_str) {
            Ok(u) => u,
            Err(_) => return std::ptr::null_mut(),
        };
        let title_opt = if title.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(title) }
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
            )
        };
        let completed_opt = match completed {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };
        let input = UpdateTodo {
            title: title_opt,
            completed: completed_opt,
        };
        match client.inner.build_update_todo(uuid, &input) {
            Ok(req) => FfiHttpRequest::from_core(req),
            Err(_) => std::ptr::null_mut(),
        }
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Build an HTTP request for deleting a todo by id.
///
/// Returns null if `client` or `id` is null, or if `id` is not a valid UUID.
#[unsafe(no_mangle)]
pub extern "C" fn todo_build_delete_todo(
    client: *const FfiTodoClient,
    id: *const c_char,
) -> *mut FfiHttpRequest {
    catch_unwind(|| {
        if client.is_null() || id.is_null() {
            return std::ptr::null_mut();
        }
        let client = unsafe { &*client };
        let id_str = unsafe { CStr::from_ptr(id) }.to_str().unwrap_or("");
        let uuid = match uuid::Uuid::parse_str(id_str) {
            Ok(u) => u,
            Err(_) => return std::ptr::null_mut(),
        };
        let req = client.inner.build_delete_todo(uuid);
        FfiHttpRequest::from_core(req)
    })
    .unwrap_or(std::ptr::null_mut())
}

// ---------------------------------------------------------------------------
// Parse response functions
// ---------------------------------------------------------------------------

/// Convert an `FfiHttpResponse` to a core `HttpResponse`.
///
/// Returns `None` if the body pointer is null (treated as empty string is
/// valid, but the response pointer itself being null is caught by callers).
fn ffi_response_to_core(resp: &FfiHttpResponse) -> HttpResponse {
    let body = if resp.body.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(resp.body) }
            .to_str()
            .unwrap_or("")
            .to_string()
    };
    HttpResponse {
        status: resp.status,
        headers: Vec::new(),
        body,
    }
}

/// Parse an HTTP response from a list-todos request.
///
/// Returns a result with `data_tag = TodoList` on success.
#[unsafe(no_mangle)]
pub extern "C" fn todo_parse_list_todos(
    client: *const FfiTodoClient,
    response: *const FfiHttpResponse,
) -> *mut FfiTodoResult {
    catch_unwind(|| {
        if client.is_null() {
            return FfiTodoResult::null_arg("client");
        }
        if response.is_null() {
            return FfiTodoResult::null_arg("response");
        }
        let client = unsafe { &*client };
        let resp = unsafe { &*response };
        let core_resp = ffi_response_to_core(resp);
        match client.inner.parse_list_todos(core_resp) {
            Ok(todos) => FfiTodoResult::ok_todo_list(todos),
            Err(e) => FfiTodoResult::from_error(e),
        }
    })
    .unwrap_or_else(|_| FfiTodoResult::panic("panic in todo_parse_list_todos"))
}

/// Parse an HTTP response from a get-todo request.
///
/// Returns a result with `data_tag = Todo` on success.
#[unsafe(no_mangle)]
pub extern "C" fn todo_parse_get_todo(
    client: *const FfiTodoClient,
    response: *const FfiHttpResponse,
) -> *mut FfiTodoResult {
    catch_unwind(|| {
        if client.is_null() {
            return FfiTodoResult::null_arg("client");
        }
        if response.is_null() {
            return FfiTodoResult::null_arg("response");
        }
        let client = unsafe { &*client };
        let resp = unsafe { &*response };
        let core_resp = ffi_response_to_core(resp);
        match client.inner.parse_get_todo(core_resp) {
            Ok(todo) => FfiTodoResult::ok_todo(todo),
            Err(e) => FfiTodoResult::from_error(e),
        }
    })
    .unwrap_or_else(|_| FfiTodoResult::panic("panic in todo_parse_get_todo"))
}

/// Parse an HTTP response from a create-todo request.
///
/// Returns a result with `data_tag = Todo` on success (status 201).
#[unsafe(no_mangle)]
pub extern "C" fn todo_parse_create_todo(
    client: *const FfiTodoClient,
    response: *const FfiHttpResponse,
) -> *mut FfiTodoResult {
    catch_unwind(|| {
        if client.is_null() {
            return FfiTodoResult::null_arg("client");
        }
        if response.is_null() {
            return FfiTodoResult::null_arg("response");
        }
        let client = unsafe { &*client };
        let resp = unsafe { &*response };
        let core_resp = ffi_response_to_core(resp);
        match client.inner.parse_create_todo(core_resp) {
            Ok(todo) => FfiTodoResult::ok_todo(todo),
            Err(e) => FfiTodoResult::from_error(e),
        }
    })
    .unwrap_or_else(|_| FfiTodoResult::panic("panic in todo_parse_create_todo"))
}

/// Parse an HTTP response from an update-todo request.
///
/// Returns a result with `data_tag = Todo` on success.
#[unsafe(no_mangle)]
pub extern "C" fn todo_parse_update_todo(
    client: *const FfiTodoClient,
    response: *const FfiHttpResponse,
) -> *mut FfiTodoResult {
    catch_unwind(|| {
        if client.is_null() {
            return FfiTodoResult::null_arg("client");
        }
        if response.is_null() {
            return FfiTodoResult::null_arg("response");
        }
        let client = unsafe { &*client };
        let resp = unsafe { &*response };
        let core_resp = ffi_response_to_core(resp);
        match client.inner.parse_update_todo(core_resp) {
            Ok(todo) => FfiTodoResult::ok_todo(todo),
            Err(e) => FfiTodoResult::from_error(e),
        }
    })
    .unwrap_or_else(|_| FfiTodoResult::panic("panic in todo_parse_update_todo"))
}

/// Parse an HTTP response from a delete-todo request.
///
/// Returns a result with `data_tag = None` on success (status 204).
#[unsafe(no_mangle)]
pub extern "C" fn todo_parse_delete_todo(
    client: *const FfiTodoClient,
    response: *const FfiHttpResponse,
) -> *mut FfiTodoResult {
    catch_unwind(|| {
        if client.is_null() {
            return FfiTodoResult::null_arg("client");
        }
        if response.is_null() {
            return FfiTodoResult::null_arg("response");
        }
        let client = unsafe { &*client };
        let resp = unsafe { &*response };
        let core_resp = ffi_response_to_core(resp);
        match client.inner.parse_delete_todo(core_resp) {
            Ok(()) => FfiTodoResult::ok_empty(),
            Err(e) => FfiTodoResult::from_error(e),
        }
    })
    .unwrap_or_else(|_| FfiTodoResult::panic("panic in todo_parse_delete_todo"))
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Free an `FfiHttpRequest` returned by any `todo_build_*` function.
/// Safe to call with null.
#[unsafe(no_mangle)]
pub extern "C" fn todo_free_request(req: *mut FfiHttpRequest) {
    if req.is_null() {
        return;
    }
    let _ = catch_unwind(|| {
        let req = unsafe { Box::from_raw(req) };
        if !req.path.is_null() {
            drop(unsafe { CString::from_raw(req.path) });
        }
        if !req.body.is_null() {
            drop(unsafe { CString::from_raw(req.body) });
        }
        if !req.headers.is_null() && req.headers_len > 0 {
            let headers = unsafe {
                Vec::from_raw_parts(req.headers, req.headers_len as usize, req.headers_len as usize)
            };
            for h in headers {
                if !h.key.is_null() {
                    drop(unsafe { CString::from_raw(h.key) });
                }
                if !h.value.is_null() {
                    drop(unsafe { CString::from_raw(h.value) });
                }
            }
        }
    });
}

/// Free an `FfiTodoResult` returned by any `todo_parse_*` function.
/// Safe to call with null. Uses `data_tag` to determine what `data` points to.
#[unsafe(no_mangle)]
pub extern "C" fn todo_free_result(result: *mut FfiTodoResult) {
    if result.is_null() {
        return;
    }
    let _ = catch_unwind(|| {
        let result = unsafe { Box::from_raw(result) };
        if !result.error_message.is_null() {
            drop(unsafe { CString::from_raw(result.error_message) });
        }
        if !result.data.is_null() {
            match result.data_tag {
                FfiDataTag::Todo => {
                    let todo = unsafe { Box::from_raw(result.data as *mut FfiTodo) };
                    free_ffi_todo_fields(&todo);
                }
                FfiDataTag::TodoList => {
                    let list = unsafe { Box::from_raw(result.data as *mut FfiTodoList) };
                    if !list.items.is_null() && list.len > 0 {
                        let items = unsafe {
                            Vec::from_raw_parts(
                                list.items,
                                list.len as usize,
                                list.len as usize,
                            )
                        };
                        for item in &items {
                            free_ffi_todo_fields(item);
                        }
                    }
                }
                FfiDataTag::None => {}
            }
        }
    });
}

/// Free the C-string fields of an `FfiTodo` (but not the struct itself).
fn free_ffi_todo_fields(todo: &FfiTodo) {
    if !todo.id.is_null() {
        drop(unsafe { CString::from_raw(todo.id) });
    }
    if !todo.title.is_null() {
        drop(unsafe { CString::from_raw(todo.title) });
    }
}

/// Free a C string allocated by this library. Safe to call with null.
#[unsafe(no_mangle)]
pub extern "C" fn todo_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = catch_unwind(|| {
            drop(unsafe { CString::from_raw(s) });
        });
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn client_new_and_free() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        assert!(!client.is_null());
        todo_client_free(client);
    }

    #[test]
    fn client_new_null_returns_null() {
        let client = todo_client_new(std::ptr::null());
        assert!(client.is_null());
    }

    #[test]
    fn client_free_null_is_safe() {
        todo_client_free(std::ptr::null_mut());
    }

    #[test]
    fn build_list_todos_returns_correct_request() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let req = todo_build_list_todos(client);
        assert!(!req.is_null());

        let req_ref = unsafe { &*req };
        assert!(matches!(req_ref.method, FfiHttpMethod::Get));

        let path = unsafe { CStr::from_ptr(req_ref.path) }.to_str().unwrap();
        assert_eq!(path, "http://localhost:3000/todos");

        assert!(req_ref.body.is_null());
        assert_eq!(req_ref.headers_len, 0);

        todo_free_request(req);
        todo_client_free(client);
    }

    #[test]
    fn build_list_todos_null_client_returns_null() {
        let req = todo_build_list_todos(std::ptr::null());
        assert!(req.is_null());
    }

    #[test]
    fn build_get_todo_valid_uuid() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let id = CString::new("00000000-0000-0000-0000-000000000001").unwrap();
        let req = todo_build_get_todo(client, id.as_ptr());
        assert!(!req.is_null());

        let req_ref = unsafe { &*req };
        let path = unsafe { CStr::from_ptr(req_ref.path) }.to_str().unwrap();
        assert_eq!(
            path,
            "http://localhost:3000/todos/00000000-0000-0000-0000-000000000001"
        );
        assert!(matches!(req_ref.method, FfiHttpMethod::Get));

        todo_free_request(req);
        todo_client_free(client);
    }

    #[test]
    fn build_get_todo_invalid_uuid_returns_null() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let id = CString::new("not-a-uuid").unwrap();
        let req = todo_build_get_todo(client, id.as_ptr());
        assert!(req.is_null());
        todo_client_free(client);
    }

    #[test]
    fn build_create_todo_produces_post_with_json_body() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let title = CString::new("Buy milk").unwrap();
        let req = todo_build_create_todo(client, title.as_ptr(), false);
        assert!(!req.is_null());

        let req_ref = unsafe { &*req };
        assert!(matches!(req_ref.method, FfiHttpMethod::Post));
        assert_eq!(req_ref.headers_len, 1);
        assert!(!req_ref.body.is_null());

        let body_str = unsafe { CStr::from_ptr(req_ref.body) }.to_str().unwrap();
        let body: serde_json::Value = serde_json::from_str(body_str).unwrap();
        assert_eq!(body["title"], "Buy milk");
        assert_eq!(body["completed"], false);

        todo_free_request(req);
        todo_client_free(client);
    }

    #[test]
    fn build_update_todo_title_only() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let id = CString::new("00000000-0000-0000-0000-000000000001").unwrap();
        let title = CString::new("New title").unwrap();
        let req = todo_build_update_todo(client, id.as_ptr(), title.as_ptr(), -1);
        assert!(!req.is_null());

        let req_ref = unsafe { &*req };
        assert!(matches!(req_ref.method, FfiHttpMethod::Put));
        let body_str = unsafe { CStr::from_ptr(req_ref.body) }.to_str().unwrap();
        let body: serde_json::Value = serde_json::from_str(body_str).unwrap();
        assert_eq!(body["title"], "New title");
        assert!(body.get("completed").is_none());

        todo_free_request(req);
        todo_client_free(client);
    }

    #[test]
    fn build_update_todo_completed_only() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let id = CString::new("00000000-0000-0000-0000-000000000001").unwrap();
        let req = todo_build_update_todo(client, id.as_ptr(), std::ptr::null(), 1);
        assert!(!req.is_null());

        let req_ref = unsafe { &*req };
        let body_str = unsafe { CStr::from_ptr(req_ref.body) }.to_str().unwrap();
        let body: serde_json::Value = serde_json::from_str(body_str).unwrap();
        assert!(body.get("title").is_none());
        assert_eq!(body["completed"], true);

        todo_free_request(req);
        todo_client_free(client);
    }

    #[test]
    fn build_delete_todo_valid_uuid() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let id = CString::new("00000000-0000-0000-0000-000000000001").unwrap();
        let req = todo_build_delete_todo(client, id.as_ptr());
        assert!(!req.is_null());

        let req_ref = unsafe { &*req };
        assert!(matches!(req_ref.method, FfiHttpMethod::Delete));

        todo_free_request(req);
        todo_client_free(client);
    }

    #[test]
    fn parse_list_todos_empty() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let body = CString::new("[]").unwrap();
        let resp = FfiHttpResponse {
            status: 200,
            body: body.as_ptr(),
        };
        let result = todo_parse_list_todos(client, &resp);
        assert!(!result.is_null());

        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::Ok));
        assert!(r.error_message.is_null());
        assert!(matches!(r.data_tag, FfiDataTag::TodoList));

        let list = unsafe { &*(r.data as *const FfiTodoList) };
        assert_eq!(list.len, 0);

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn parse_list_todos_two_items() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let body = CString::new(
            r#"[
                {"id":"00000000-0000-0000-0000-000000000001","title":"First","completed":false},
                {"id":"00000000-0000-0000-0000-000000000002","title":"Second","completed":true}
            ]"#,
        )
        .unwrap();
        let resp = FfiHttpResponse {
            status: 200,
            body: body.as_ptr(),
        };
        let result = todo_parse_list_todos(client, &resp);
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::Ok));
        assert!(matches!(r.data_tag, FfiDataTag::TodoList));

        let list = unsafe { &*(r.data as *const FfiTodoList) };
        assert_eq!(list.len, 2);

        let items = unsafe { std::slice::from_raw_parts(list.items, list.len as usize) };
        let title0 = unsafe { CStr::from_ptr(items[0].title) }.to_str().unwrap();
        assert_eq!(title0, "First");
        assert!(!items[0].completed);

        let title1 = unsafe { CStr::from_ptr(items[1].title) }.to_str().unwrap();
        assert_eq!(title1, "Second");
        assert!(items[1].completed);

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn parse_delete_todo_success() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let body = CString::new("").unwrap();
        let resp = FfiHttpResponse {
            status: 204,
            body: body.as_ptr(),
        };
        let result = todo_parse_delete_todo(client, &resp);
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::Ok));
        assert!(matches!(r.data_tag, FfiDataTag::None));
        assert!(r.data.is_null());

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn parse_delete_todo_not_found() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let body = CString::new("").unwrap();
        let resp = FfiHttpResponse {
            status: 404,
            body: body.as_ptr(),
        };
        let result = todo_parse_delete_todo(client, &resp);
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::NotFound));
        assert!(!r.error_message.is_null());

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn parse_null_client_returns_null_arg() {
        let body = CString::new("[]").unwrap();
        let resp = FfiHttpResponse {
            status: 200,
            body: body.as_ptr(),
        };
        let result = todo_parse_list_todos(std::ptr::null(), &resp);
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::NullArg));

        todo_free_result(result);
    }

    #[test]
    fn parse_null_response_returns_null_arg() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let result = todo_parse_list_todos(client, std::ptr::null());
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::NullArg));

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn parse_get_todo_success() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let body = CString::new(
            r#"{"id":"00000000-0000-0000-0000-000000000001","title":"Test","completed":false}"#,
        )
        .unwrap();
        let resp = FfiHttpResponse {
            status: 200,
            body: body.as_ptr(),
        };
        let result = todo_parse_get_todo(client, &resp);
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::Ok));
        assert!(matches!(r.data_tag, FfiDataTag::Todo));

        let todo = unsafe { &*(r.data as *const FfiTodo) };
        let title = unsafe { CStr::from_ptr(todo.title) }.to_str().unwrap();
        assert_eq!(title, "Test");
        assert!(!todo.completed);

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn parse_get_todo_not_found() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let body = CString::new("").unwrap();
        let resp = FfiHttpResponse {
            status: 404,
            body: body.as_ptr(),
        };
        let result = todo_parse_get_todo(client, &resp);
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::NotFound));

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn parse_create_todo_success() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let body = CString::new(
            r#"{"id":"00000000-0000-0000-0000-000000000001","title":"New","completed":false}"#,
        )
        .unwrap();
        let resp = FfiHttpResponse {
            status: 201,
            body: body.as_ptr(),
        };
        let result = todo_parse_create_todo(client, &resp);
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::Ok));
        assert!(matches!(r.data_tag, FfiDataTag::Todo));

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn parse_update_todo_success() {
        let url = CString::new("http://localhost:3000").unwrap();
        let client = todo_client_new(url.as_ptr());
        let body = CString::new(
            r#"{"id":"00000000-0000-0000-0000-000000000001","title":"Updated","completed":true}"#,
        )
        .unwrap();
        let resp = FfiHttpResponse {
            status: 200,
            body: body.as_ptr(),
        };
        let result = todo_parse_update_todo(client, &resp);
        let r = unsafe { &*result };
        assert!(matches!(r.error_code, FfiErrorCode::Ok));
        assert!(matches!(r.data_tag, FfiDataTag::Todo));

        let todo = unsafe { &*(r.data as *const FfiTodo) };
        let title = unsafe { CStr::from_ptr(todo.title) }.to_str().unwrap();
        assert_eq!(title, "Updated");
        assert!(todo.completed);

        todo_free_result(result);
        todo_client_free(client);
    }

    #[test]
    fn free_request_null_is_safe() {
        todo_free_request(std::ptr::null_mut());
    }

    #[test]
    fn free_result_null_is_safe() {
        todo_free_result(std::ptr::null_mut());
    }

    #[test]
    fn free_string_null_is_safe() {
        todo_free_string(std::ptr::null_mut());
    }
}
