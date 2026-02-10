//! `#[repr(C)]` types for the FFI boundary.
//!
//! # Design
//! Each type mirrors a core type but uses C-compatible representations:
//! `*mut c_char` instead of `String`, raw pointers instead of `Vec`, and
//! tagged enums with explicit discriminants. Conversion functions live here
//! to keep `lib.rs` focused on the `extern "C"` surface.

use std::ffi::CString;
use std::os::raw::c_char;

use todo_core::error::ApiError;
use todo_core::http::HttpMethod;

/// Opaque handle to a `TodoClient`. C callers receive a pointer to this
/// and pass it back into every FFI function.
pub struct FfiTodoClient {
    pub(crate) inner: todo_core::TodoClient,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// HTTP method as a C enum.
#[repr(C)]
pub enum FfiHttpMethod {
    Get = 0,
    Post = 1,
    Put = 2,
    Delete = 3,
}

impl From<HttpMethod> for FfiHttpMethod {
    fn from(m: HttpMethod) -> Self {
        match m {
            HttpMethod::Get => FfiHttpMethod::Get,
            HttpMethod::Post => FfiHttpMethod::Post,
            HttpMethod::Put => FfiHttpMethod::Put,
            HttpMethod::Delete => FfiHttpMethod::Delete,
        }
    }
}

/// A single HTTP header as a key-value pair of C strings.
#[repr(C)]
pub struct FfiHeader {
    pub key: *mut c_char,
    pub value: *mut c_char,
}

/// An HTTP request described as C-compatible plain data.
///
/// Built by `todo_build_*` functions. The C caller executes the request
/// and passes the response back through `todo_parse_*`.
#[repr(C)]
pub struct FfiHttpRequest {
    pub method: FfiHttpMethod,
    pub path: *mut c_char,
    pub headers: *mut FfiHeader,
    pub headers_len: u32,
    pub body: *mut c_char,
}

impl FfiHttpRequest {
    /// Convert a core `HttpRequest` into a heap-allocated `FfiHttpRequest`.
    pub(crate) fn from_core(req: todo_core::HttpRequest) -> *mut Self {
        let path = CString::new(req.path).unwrap().into_raw();
        let body = match req.body {
            Some(b) => CString::new(b).unwrap().into_raw(),
            None => std::ptr::null_mut(),
        };

        let headers_len = req.headers.len() as u32;
        let headers = if req.headers.is_empty() {
            std::ptr::null_mut()
        } else {
            let mut ffi_headers: Vec<FfiHeader> = req
                .headers
                .into_iter()
                .map(|(k, v)| FfiHeader {
                    key: CString::new(k).unwrap().into_raw(),
                    value: CString::new(v).unwrap().into_raw(),
                })
                .collect();
            let ptr = ffi_headers.as_mut_ptr();
            std::mem::forget(ffi_headers);
            ptr
        };

        let ffi_req = Box::new(FfiHttpRequest {
            method: req.method.into(),
            path,
            headers,
            headers_len,
            body,
        });
        Box::into_raw(ffi_req)
    }
}

// ---------------------------------------------------------------------------
// Response input (caller-provided, not heap-allocated by us)
// ---------------------------------------------------------------------------

/// An HTTP response described as C-compatible plain data.
///
/// The C caller constructs this on the stack after executing an HTTP request,
/// then passes a pointer to a `todo_parse_*` function. The FFI layer reads
/// but does not free these fields.
#[repr(C)]
pub struct FfiHttpResponse {
    pub status: u16,
    pub body: *const c_char,
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Error codes returned in `FfiTodoResult`.
#[repr(C)]
pub enum FfiErrorCode {
    Ok = 0,
    NotFound = 1,
    Http = 2,
    Deserialization = 3,
    Serialization = 4,
    Panic = 5,
    NullArg = 6,
}

/// Tag that tells `todo_free_result` what `FfiTodoResult::data` points to.
#[repr(C)]
pub enum FfiDataTag {
    None = 0,
    Todo = 1,
    TodoList = 2,
}

/// A single todo item exposed to C.
#[repr(C)]
pub struct FfiTodo {
    pub id: *mut c_char,
    pub title: *mut c_char,
    pub completed: bool,
}

/// A list of todo items exposed to C.
#[repr(C)]
pub struct FfiTodoList {
    pub items: *mut FfiTodo,
    pub len: u32,
}

/// Result envelope for all parse operations.
///
/// On success `error_code` is `Ok`, `error_message` is null, and `data`
/// points to the parsed payload (tagged by `data_tag`).
/// On failure `error_code` describes the category, `error_message` is a
/// human-readable C string, and `data` is null.
#[repr(C)]
pub struct FfiTodoResult {
    pub error_code: FfiErrorCode,
    pub error_message: *mut c_char,
    pub http_status: u16,
    pub data_tag: FfiDataTag,
    pub data: *mut std::ffi::c_void,
}

impl FfiTodoResult {
    /// Build a success result carrying a single `FfiTodo`.
    pub(crate) fn ok_todo(todo: todo_core::Todo) -> *mut Self {
        let ffi_todo = Box::new(FfiTodo {
            id: CString::new(todo.id.to_string()).unwrap().into_raw(),
            title: CString::new(todo.title).unwrap().into_raw(),
            completed: todo.completed,
        });
        let result = Box::new(FfiTodoResult {
            error_code: FfiErrorCode::Ok,
            error_message: std::ptr::null_mut(),
            http_status: 0,
            data_tag: FfiDataTag::Todo,
            data: Box::into_raw(ffi_todo) as *mut std::ffi::c_void,
        });
        Box::into_raw(result)
    }

    /// Build a success result carrying a `FfiTodoList`.
    pub(crate) fn ok_todo_list(todos: Vec<todo_core::Todo>) -> *mut Self {
        let len = todos.len() as u32;
        let mut ffi_todos: Vec<FfiTodo> = todos
            .into_iter()
            .map(|t| FfiTodo {
                id: CString::new(t.id.to_string()).unwrap().into_raw(),
                title: CString::new(t.title).unwrap().into_raw(),
                completed: t.completed,
            })
            .collect();

        let items = if ffi_todos.is_empty() {
            std::ptr::null_mut()
        } else {
            let ptr = ffi_todos.as_mut_ptr();
            std::mem::forget(ffi_todos);
            ptr
        };

        let ffi_list = Box::new(FfiTodoList { items, len });
        let result = Box::new(FfiTodoResult {
            error_code: FfiErrorCode::Ok,
            error_message: std::ptr::null_mut(),
            http_status: 0,
            data_tag: FfiDataTag::TodoList,
            data: Box::into_raw(ffi_list) as *mut std::ffi::c_void,
        });
        Box::into_raw(result)
    }

    /// Build a success result with no data payload (e.g. delete).
    pub(crate) fn ok_empty() -> *mut Self {
        let result = Box::new(FfiTodoResult {
            error_code: FfiErrorCode::Ok,
            error_message: std::ptr::null_mut(),
            http_status: 0,
            data_tag: FfiDataTag::None,
            data: std::ptr::null_mut(),
        });
        Box::into_raw(result)
    }

    /// Build an error result from an `ApiError`.
    pub(crate) fn from_error(err: ApiError) -> *mut Self {
        let (error_code, http_status, msg) = match &err {
            ApiError::NotFound => (FfiErrorCode::NotFound, 404u16, err.to_string()),
            ApiError::HttpError { status, .. } => {
                (FfiErrorCode::Http, *status, err.to_string())
            }
            ApiError::DeserializationError(_) => {
                (FfiErrorCode::Deserialization, 0, err.to_string())
            }
            ApiError::SerializationError(_) => {
                (FfiErrorCode::Serialization, 0, err.to_string())
            }
        };

        let result = Box::new(FfiTodoResult {
            error_code,
            error_message: CString::new(msg).unwrap().into_raw(),
            http_status,
            data_tag: FfiDataTag::None,
            data: std::ptr::null_mut(),
        });
        Box::into_raw(result)
    }

    /// Build an error result for a null argument.
    pub(crate) fn null_arg(name: &str) -> *mut Self {
        let msg = format!("null argument: {name}");
        let result = Box::new(FfiTodoResult {
            error_code: FfiErrorCode::NullArg,
            error_message: CString::new(msg).unwrap().into_raw(),
            http_status: 0,
            data_tag: FfiDataTag::None,
            data: std::ptr::null_mut(),
        });
        Box::into_raw(result)
    }

    /// Build an error result for a caught panic.
    pub(crate) fn panic(msg: &str) -> *mut Self {
        let result = Box::new(FfiTodoResult {
            error_code: FfiErrorCode::Panic,
            error_message: CString::new(msg).unwrap_or_default().into_raw(),
            http_status: 0,
            data_tag: FfiDataTag::None,
            data: std::ptr::null_mut(),
        });
        Box::into_raw(result)
    }
}
