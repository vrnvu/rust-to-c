# rust-to-c

Cross-language HTTP API client: Rust core → C ABI → language wrappers.

## 1. Goals & non-goals

### Goals

* Rust HTTP API client for a CRUD todo-list service
* Stable C ABI via `extern "C"`
* Language wrappers:
  * TypeScript (Node.js via N-API + Browser via WASM)
  * Java (JNI + Android NDK)
  * C# (P/Invoke)
  * C++ (thin RAII wrapper)
* Mock HTTP server (Rust/Axum) for testing
* Cross-language integration tests using shared test vectors
* Host-does-IO pattern: Rust core builds requests, host executes HTTP

### Non-goals (out of scope for MVP)

* Auth / OAuth / token management
* UI components
* Production-grade error handling / retries
* High-level idiomatic APIs per language

---

## 2. High-level architecture

```
┌──────────────────────────┐
│    Mock Server (Axum)    │
│  CRUD Todo REST API      │
│  GET/POST/PUT/DELETE     │
└─────────────▲────────────┘
              │ HTTP
              │
┌─────────────┴────────────┐
│       Rust Core          │
│--------------------------│
│ API client logic         │
│ Request builder          │
│ Response parser          │
│ DTOs / domain types      │
│ Error classification     │
└─────────────▲────────────┘
              │
      Stable C ABI (extern "C")
              │
┌─────────────┼────────────────────┐
│             │                    │
│   WASM    Native    Native    Native
│   (Web)   (Java)   (C#)     (C++)
│             │                    │
└──▲──────────┼──────────────▲─────┘
   │          │              │
 TS wrapper  JNI binding   C# / C++ wrapper
   │          │              │
Integration tests (shared test vectors)
```

Key principle:

> **The Rust core owns all API logic. Hosts only perform HTTP I/O.**

---

## 3. Host-does-IO pattern

The Rust core is **synchronous and does no I/O**. All flows follow this loop:

1. Host calls Rust core (e.g. `create_todo(todo)`)
2. Core returns an `HttpRequest` struct (method, URL, headers, body)
3. Host executes HTTP however it wants (fetch, OkHttp, HttpClient, etc.)
4. Host feeds back an `HttpResponse` struct (status, headers, body)
5. Core parses response, returns typed result or error

Each language wrapper decides its own async strategy:

* **JS/TS**: `async/await` + `fetch`
* **Java/Android**: `CompletableFuture` or coroutines
* **C#**: `Task` / `async`
* **C++**: callbacks, futures, or blocking

---

## 4. Mock server (Axum)

Simple REST API for a todo list:

| Method   | Path          | Description       |
|----------|---------------|-------------------|
| `GET`    | `/todos`      | List all todos    |
| `GET`    | `/todos/:id`  | Get a todo by ID  |
| `POST`   | `/todos`      | Create a todo     |
| `PUT`    | `/todos/:id`  | Update a todo     |
| `DELETE` | `/todos/:id`  | Delete a todo     |

Todo schema:

```json
{
  "id": "uuid",
  "title": "string",
  "completed": false
}
```

---

## 5. Rust core design

### Crate structure

```
Cargo.toml (workspace)
├─ core/
│  ├─ src/
│  │  ├─ lib.rs
│  │  ├─ types.rs        # Todo, CreateTodo, UpdateTodo DTOs
│  │  ├─ client.rs       # API client (builds HttpRequest, parses HttpResponse)
│  │  ├─ http.rs         # HttpRequest, HttpResponse, HttpMethod
│  │  └─ error.rs        # ApiError enum
│  └─ Cargo.toml
├─ ffi/
│  ├─ src/
│  │  ├─ lib.rs
│  │  └─ c_api.rs        # extern "C" functions
│  ├─ include/
│  │  └─ todo_client.h   # Generated C header
│  └─ Cargo.toml
├─ mock-server/
│  ├─ src/
│  │  └─ main.rs
│  └─ Cargo.toml
└─ test-vectors/
   ├─ create-todo.json
   ├─ list-todos.json
   ├─ update-todo.json
   └─ delete-todo.json
```

### Core design rules

* No async
* No networking (host-does-IO)
* No panics across FFI (`catch_unwind`)
* Deterministic behavior for testing
* All serialization via `serde`

---

## 6. C ABI contract

### Design principles

* Opaque pointers for client state
* Explicit memory ownership
* Flat data structures (no nested pointers where avoidable)
* All strings returned by core are freed via `todo_free_string()`

### Conceptual ABI surface

```c
typedef struct TodoClient TodoClient;

// Lifecycle
TodoClient* todo_client_new(const char* base_url);
void todo_client_free(TodoClient* client);

// Operations — each returns an HttpRequest the host must execute
HttpRequest* todo_list_todos(TodoClient* client);
HttpRequest* todo_get_todo(TodoClient* client, const char* id);
HttpRequest* todo_create_todo(TodoClient* client, const char* title);
HttpRequest* todo_update_todo(TodoClient* client, const char* id, const char* title, int completed);
HttpRequest* todo_delete_todo(TodoClient* client, const char* id);

// Feed HTTP response back, get typed result
TodoResult* todo_handle_response(TodoClient* client, const HttpResponse* response);

// Memory
void todo_free_string(char* s);
void todo_free_request(HttpRequest* req);
void todo_free_result(TodoResult* result);
```

---

## 7. Platform bindings

### TypeScript — Node.js (N-API)
* Native addon loading the compiled Rust library
* Async wrapper using `fetch` or `node:http`

### TypeScript — Browser (WASM)
* Compile Rust core to WASM via `wasm-pack`
* JS wrapper uses `fetch` for HTTP

### Java (JNI + Android)
* JNI bindings to native Rust library
* Thin Java wrapper with `CompletableFuture` API
* Android: same JNI, cross-compiled via NDK

### C# (P/Invoke)
* P/Invoke bindings auto-generated from C header
* Thin wrapper with `Task`-based async API

### C++ (Native)
* Thin RAII wrapper around C ABI
* Header-only or minimal `.cpp`

---

## 8. Integration testing strategy

### Test vectors (`test-vectors/`)

JSON files with:
* Deterministic inputs (todo data)
* Expected `HttpRequest` outputs (URL, method, headers, body)
* Simulated `HttpResponse` inputs
* Expected parsed results

### Per-language tests

Each language wrapper:
1. Loads the same test vectors
2. Calls the Rust core to build requests
3. Asserts request matches expected
4. Feeds simulated responses
5. Asserts parsed results match expected

This proves: **given the same inputs, every language produces identical behavior.**

---

## 9. MVP milestones

### Phase 1 — Mock server

**Milestone 1 — Mock server**
* Axum-based CRUD todo API
* In-memory storage
* Runs locally

### Phase 2 — Rust core + integration tests

**Milestone 2 — Rust core**
* DTOs and API client (Todo, CreateTodo, UpdateTodo)
* HttpRequest / HttpResponse types
* Request builder + response parser
* Error types
* Rust unit tests with test vectors

**Milestone 3 — Integration tests (Rust core against mock server)**
* Rust integration test: core → real HTTP → mock server
* Proves the full roundtrip works end-to-end

### Phase 3 — C ABI

**Milestone 4 — C ABI**
* `extern "C"` interface
* C header generation (cbindgen)
* Memory ownership + free functions
* C ABI tests in Rust
* C integration test: C ABI → real HTTP → mock server

### Phase 4 — Host wrappers

**Milestone 5 — TypeScript bindings**
* WASM build + browser JS wrapper
* Node.js N-API addon
* Integration tests using test vectors + mock server

**Milestone 6 — Java binding**
* JNI bindings
* Java wrapper
* Android cross-compilation (NDK)
* Integration tests

**Milestone 7 — C# binding**
* P/Invoke bindings
* C# wrapper
* Integration tests

**Milestone 8 — C++ binding**
* RAII wrapper
* Integration tests
* Confirm parity with Rust tests

---

## Design guidance

> Favor correctness and explicitness over ergonomics.
> All API calls must go through the Rust core.
> If logic is duplicated in a wrapper, it is a bug.
> The core does zero I/O. Always.
