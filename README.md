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

----

PT 2

Got it. Below is a clean, explicit, Claude-friendly structure summary you can drop straight into your README.
It’s written as instructions + plan, not prose, so another LLM (or human) can adopt the architecture without guessing intent.

You can paste this verbatim under something like “Project Structure & Design Rationale”.

⸻

Project structure summary (authoritative)

Purpose

This repository implements a cross-language authentication protocol client using a Rust core + stable C ABI architecture.

The goal is not to expose raw REST endpoints, but to enforce a stateful authentication flow (OAuth / OpenID–like) consistently across platforms:
	•	Web (browser)
	•	Mobile (iOS / Android)
	•	Consoles (PS5, Xbox)
	•	Native clients (C++, C#, Java)

All protocol logic lives in Rust.
All I/O and async execution is delegated to the host platform.

⸻

Core design principles
	1.	The Rust core owns the protocol
	•	All flow logic, state transitions, validation, and parsing are implemented once.
	•	Hosts are not allowed to re-implement or shortcut protocol steps.
	2.	Host-does-I/O
	•	The Rust core performs no networking and no async.
	•	The core only describes HTTP requests and consumes HTTP responses.
	•	Hosts execute HTTP using platform-native tools (fetch, OkHttp, HttpClient, etc.).
	3.	Stable C ABI as the universal boundary
	•	Rust is exposed via extern "C" functions.
	•	All higher-level language bindings are thin wrappers over the C ABI.
	•	No language binding contains protocol logic.
	4.	Stateful protocol enforcement
	•	The authentication flow is modeled as an explicit state machine.
	•	Invalid transitions are rejected by the core.
	•	Correctness is enforced by construction, not by documentation.
	5.	Deterministic and testable
	•	Mock backend responses are deterministic.
	•	Shared test vectors guarantee identical behavior across all languages.

⸻

Demo protocol: minimal OAuth-shaped device flow

This repository uses a minimal, mocked, OAuth-shaped device authorization flow to demonstrate the architecture.

This is not production OAuth.
Cryptography and security features are intentionally simplified.

Why this demo exists
	•	CRUD APIs (e.g. todo lists) do not justify a protocol-enforcing core.
	•	Authentication flows are:
	•	multi-step
	•	stateful
	•	easy to misimplement
	•	This makes them the correct demonstration domain.

⸻

Protocol model (simplified)

States

Init
 → DeviceCodeIssued
 → Authorized
 → TokenIssued

Rules
	•	Calls are only valid in specific states.
	•	The core rejects invalid sequences.
	•	The host cannot skip steps or fabricate results.

⸻

Flow overview

1. Start device authorization

Rust core → returns HttpRequest
Host → executes HTTP
Host → feeds HttpResponse back

Response includes:
	•	device_code
	•	user_code
	•	verification_uri
	•	polling interval

Core transitions to DeviceCodeIssued.

⸻

2. Poll for authorization

Host repeatedly:
	•	requests polling instructions from the core
	•	executes HTTP
	•	feeds responses back

Mock server simulates:
	•	authorization_pending
	•	success after N polls
	•	optional terminal error

Core enforces:
	•	retryable vs terminal errors
	•	allowed polling behavior

⸻

3. Token issued

On success:
	•	core parses response
	•	core transitions to TokenIssued
	•	core returns a typed token result

Flow ends.

⸻

Rust workspace structure

Cargo.toml (workspace)
├─ core/
│  ├─ types.rs        # Protocol DTOs
│  ├─ state.rs        # Auth state machine
│  ├─ client.rs       # Flow orchestration
│  ├─ http.rs         # HttpRequest / HttpResponse
│  └─ error.rs        # Protocol errors
│
├─ ffi/
│  ├─ c_api.rs        # extern "C" interface
│  ├─ lib.rs
│  └─ include/
│     └─ auth_client.h
│
├─ mock-server/
│  └─ Axum-based deterministic mock auth server
│
└─ test-vectors/
   └─ JSON files describing:
      - inputs
      - expected HttpRequests
      - simulated HttpResponses
      - expected state transitions


⸻

C ABI responsibilities

The C ABI exposes:
	•	Client lifecycle management
	•	Flow initiation
	•	Request generation
	•	Response handling
	•	Explicit memory management

ABI guarantees
	•	No panics cross the FFI boundary
	•	All memory ownership is explicit
	•	All returned strings and structs have corresponding free functions
	•	ABI is stable across language bindings

⸻

Language bindings

Each language binding:
	•	Calls the C ABI only
	•	Executes HTTP using native tools
	•	Adapts async semantics to the platform
	•	Contains zero protocol logic

Bindings include:
	•	TypeScript (Browser + Node.js)
	•	Java (JNI + Android NDK)
	•	C# (P/Invoke)
	•	C++ (RAII wrapper)

⸻

Testing strategy

Shared test vectors

All languages use the same JSON test vectors to verify:
	•	request construction
	•	response parsing
	•	state transitions
	•	error handling

What this guarantees

Given the same inputs, every platform behaves identically.

If a wrapper diverges, it is a bug.

⸻

Non-goals (explicit)

This project does not aim to provide:
	•	Full OAuth or OpenID compliance
	•	PKCE, refresh tokens, expiry handling
	•	Production-grade security guarantees
	•	Idiomatic high-level SDK ergonomics

Those are intentionally out of scope for this repository.

⸻

Design rule (non-negotiable)

If protocol logic exists outside the Rust core, it is a bug.

⸻

If you want, next I can:
	•	tighten this into an even more formal “spec tone”
	•	rewrite it specifically optimized for Claude / GPT code agents
	•	or help you split this into README vs ARCHITECTURE.md

This is a strong foundation.
