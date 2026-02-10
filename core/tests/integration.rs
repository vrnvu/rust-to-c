//! Full CRUD lifecycle test against the live mock server.
//!
//! # Design
//! Starts the mock server on a random port, then exercises every core client
//! operation over real HTTP using ureq. Validates that the core's request
//! building and response parsing work end-to-end with the actual server.

use todo_core::{ApiError, CreateTodo, HttpMethod, HttpResponse, TodoClient, UpdateTodo};

/// Execute an `HttpRequest` using ureq and return an `HttpResponse`.
///
/// Disables ureq's automatic status-code-as-error behavior so 4xx/5xx
/// responses are returned as data rather than `Err`, letting the core
/// client handle status interpretation.
fn execute(req: todo_core::HttpRequest) -> HttpResponse {
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent();

    let mut response = match (req.method, req.body) {
        (HttpMethod::Get, _) => agent.get(&req.path).call(),
        (HttpMethod::Delete, _) => agent.delete(&req.path).call(),
        (HttpMethod::Post, Some(body)) => {
            agent.post(&req.path).content_type("application/json").send(body.as_bytes())
        }
        (HttpMethod::Post, None) => agent.post(&req.path).send_empty(),
        (HttpMethod::Put, Some(body)) => {
            agent.put(&req.path).content_type("application/json").send(body.as_bytes())
        }
        (HttpMethod::Put, None) => agent.put(&req.path).send_empty(),
    }
    .expect("HTTP transport error");

    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();

    HttpResponse {
        status,
        headers: Vec::new(),
        body,
    }
}

#[test]
fn crud_lifecycle() {
    // Step 1: start mock server on a random port.
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = std_listener.local_addr().unwrap();
    std_listener.set_nonblocking(true).unwrap();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
            mock_server::run(listener).await
        })
        .unwrap();
    });

    let client = TodoClient::new(&format!("http://{addr}"));

    // Step 2: list — should be empty.
    let req = client.build_list_todos();
    let todos = client.parse_list_todos(execute(req)).unwrap();
    assert!(todos.is_empty(), "expected empty list");

    // Step 3: create a todo.
    let create_input = CreateTodo {
        title: "Integration test".to_string(),
        completed: false,
    };
    let req = client.build_create_todo(&create_input).unwrap();
    let created = client.parse_create_todo(execute(req)).unwrap();
    assert_eq!(created.title, "Integration test");
    assert!(!created.completed);
    let id = created.id;

    // Step 4: get the created todo.
    let req = client.build_get_todo(id);
    let fetched = client.parse_get_todo(execute(req)).unwrap();
    assert_eq!(fetched, created);

    // Step 5: update title.
    let update_input = UpdateTodo {
        title: Some("Updated title".to_string()),
        completed: None,
    };
    let req = client.build_update_todo(id, &update_input).unwrap();
    let updated = client.parse_update_todo(execute(req)).unwrap();
    assert_eq!(updated.title, "Updated title");
    assert!(!updated.completed);

    // Step 6: update completed.
    let update_input = UpdateTodo {
        title: None,
        completed: Some(true),
    };
    let req = client.build_update_todo(id, &update_input).unwrap();
    let updated = client.parse_update_todo(execute(req)).unwrap();
    assert_eq!(updated.title, "Updated title");
    assert!(updated.completed);

    // Step 7: list — should have one item.
    let req = client.build_list_todos();
    let todos = client.parse_list_todos(execute(req)).unwrap();
    assert_eq!(todos.len(), 1);

    // Step 8: delete.
    let req = client.build_delete_todo(id);
    client.parse_delete_todo(execute(req)).unwrap();

    // Step 9: get after delete — should be NotFound.
    let req = client.build_get_todo(id);
    let err = client.parse_get_todo(execute(req)).unwrap_err();
    assert!(matches!(err, ApiError::NotFound));

    // Step 10: delete again — should be NotFound.
    let req = client.build_delete_todo(id);
    let err = client.parse_delete_todo(execute(req)).unwrap_err();
    assert!(matches!(err, ApiError::NotFound));

    // Step 11: list — should be empty again.
    let req = client.build_list_todos();
    let todos = client.parse_list_todos(execute(req)).unwrap();
    assert!(todos.is_empty(), "expected empty list after delete");
}
