## 1. Goals & non-goals (important to align Claude)

### Goals

* Implement OAuth2 / OpenID flows:

  * Authorization Code + PKCE
  * Device Authorization Flow
  * OTT (one-time token) exchange
* Support:

  * Web (JS)
  * Unity (C#)
  * Unreal (C++)
* Single authoritative implementation of:

  * PKCE
  * OAuth state machine
  * Token lifecycle (expiry, refresh)
* Enable **cross-language integration tests** using the *same core logic*
* MVP-level ergonomics, not final DX polish

### Non-goals (explicitly out of scope for MVP)

* UI components
* Token storage implementations
* Advanced error localization
* Full OpenAPI-driven SDKs
* High-level idiomatic APIs per language

---

## 2. High-level architecture

```
┌──────────────────────────┐
│        Rust Core         │
│--------------------------│
│ OAuth / OIDC engine      │
│ PKCE generation          │
│ Device flow state        │
│ Token lifecycle          │
│ Error classification     │
└─────────────▲────────────┘
              │
      Stable C ABI (extern "C")
              │
┌─────────────┼─────────────┐
│             │             │
│   WASM      │   Native    │
│   (Web)     │ (Unity/C++) │
│             │             │
└─────▲───────┴───────▲─────┘
      │               │
 JS wrapper       C# / C++ wrapper
      │               │
Integration tests (shared test vectors)
```

Key principle:

> **The Rust core owns all auth logic. Hosts only perform I/O.**

---

## 3. Rust core design

### Crate structure

```
auth-core/
├─ src/
│  ├─ lib.rs
│  ├─ pkce.rs
│  ├─ oauth.rs
│  ├─ device_flow.rs
│  ├─ token.rs
│  ├─ errors.rs
│  ├─ state_machine.rs
│  └─ ffi/
│     ├─ mod.rs
│     └─ c_api.rs
├─ Cargo.toml
```

### Core design rules

* No async exposed across the ABI
* No networking
* No storage
* No system clock access without injection
* No panics across FFI (use `catch_unwind`)
* Deterministic behavior for testing

### State-machine driven flows

All flows are modeled as explicit steps:

```rust
enum AuthStep {
  NeedAuthorizationUrl { url: String },
  NeedUserCode { code: String, verification_uri: String },
  NeedPoll { wait_seconds: u32 },
  NeedTokenExchange { request: HttpRequest },
  Completed { tokens: TokenSet },
  Error(AuthError),
}
```

The host:

* executes the step
* feeds results back into the core

This is what makes cross-language behavior identical.

---

## 4. C ABI contract (the backbone)

### Design principles

* Opaque pointers
* Explicit memory ownership
* Flat data structures
* No callbacks in MVP (poll-based is fine)
* Versioned symbols

### Example ABI surface (conceptual)

```c
typedef struct AuthContext AuthContext;

AuthContext* auth_new(const AuthConfig* cfg);
void auth_free(AuthContext* ctx);

AuthStep auth_next_step(AuthContext* ctx, AuthStepOut* out);

void auth_provide_http_response(
  AuthContext* ctx,
  const HttpResponse* response
);

void auth_tick(AuthContext* ctx, uint64_t now_ms);
```

Supporting structs:

* `AuthConfig`
* `HttpRequest`
* `HttpResponse`
* `TokenSet`
* `AuthError`

Memory rules:

* All strings returned by core are owned by core
* Host must free via `auth_free_string()`

Claude should document these rules explicitly.

---

## 5. Platform bindings

### Web (JS + WASM)

* Compile Rust core to WASM
* JS wrapper:

  * Loads WASM
  * Converts steps into:

    * `fetch`
    * browser redirects
  * Exposes Promise-based API

MVP API shape:

```js
const flow = new AuthFlow(config)
await flow.start()
```

Internally driven by the state machine.

---

### Unity (C#)

* Compile Rust core to native library
* P/Invoke bindings auto-generated from C headers
* Thin C# wrapper:

  * Converts steps into `Task`
  * Handles timers & polling
  * Leaves storage to caller

---

### Unreal (C++)

* Native Rust build
* Thin RAII wrapper around C ABI
* Expose async behavior via callbacks or futures

---

## 6. Integration testing strategy (explicit efficiency win)

This is the **killer feature** of this architecture.

### Core idea

> The same Rust core + same test vectors are exercised from **Rust, JS, C#, and C++**.

### Step 1: Test vectors

Create a `test-vectors/` directory:

```
test-vectors/
├─ pkce.json
├─ device-flow.json
├─ token-refresh.json
```

Each file contains:

* deterministic inputs
* expected outputs
* simulated HTTP responses

---

### Step 2: Rust integration tests

* Run flows using mocked HTTP responses
* Assert exact state transitions
* Assert PKCE correctness
* Assert token expiry logic

These become the **reference tests**.

---

### Step 3: Language-level integration tests

Each language:

* Loads the same test vectors
* Drives the core through the same steps
* Uses mocked HTTP transport
* Asserts identical results

Example:

* JS test runner (Vitest / Jest)
* C# NUnit tests
* C++ Catch2 tests

They all test:

> “Given the same inputs, does the core behave identically?”

This is how you prove:

* No drift
* No reimplementation bugs
* One fix fixes all platforms

---

### Step 4: End-to-end test (optional for MVP)

* Spin up a mock OAuth server
* Run:

  * Rust test
  * JS test
  * Unity test
* Verify full device + PKCE flows

---

## 7. MVP milestones (hand this to Claude)

### Milestone 1 – Rust core

* PKCE implementation
* Authorization Code + Device Flow
* Token parsing + refresh
* State machine API
* Rust integration tests

### Milestone 2 – C ABI

* Stable extern "C" interface
* Header generation
* Memory ownership docs
* C ABI tests in Rust

### Milestone 3 – Web binding

* WASM build
* JS wrapper
* JS integration tests using test vectors

### Milestone 4 – Unity binding

* Native build
* C# bindings
* Unity integration tests using same vectors

### Milestone 5 – Unreal binding

* C++ wrapper
* Basic integration test
* Confirm parity with Rust tests

---

## Final guidance to Claude (important tone-setting)

> Favor correctness and explicitness over ergonomics.
> All flows must be driven by the state machine.
> If logic is duplicated in a wrapper, it is a bug.

---

If you want, next I can:

* turn this into a **Claude-optimized prompt**
* design the exact C structs
* propose a concrete PKCE test vector
* or help you decide WASM vs native build flags

But as-is, this plan is **MVP-ready and implementation-grade**.

