use axum::http::{self, Request, StatusCode};
use http_body_util::BodyExt;
use mock_server::{app, Todo};
use tower::ServiceExt;

async fn body_json<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn body_bytes(response: axum::response::Response) -> bytes::Bytes {
    response.into_body().collect().await.unwrap().to_bytes()
}

fn json_request(method: &str, uri: &str, body: &str) -> Request<String> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .unwrap()
}

// --- list ---

#[tokio::test]
async fn list_todos_empty() {
    let app = app();
    let resp = app
        .oneshot(Request::builder().uri("/todos").body(String::new()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let todos: Vec<Todo> = body_json(resp).await;
    assert!(todos.is_empty());
}

// --- create ---

#[tokio::test]
async fn create_todo_returns_201() {
    let app = app();
    let resp = app
        .oneshot(json_request("POST", "/todos", r#"{"title":"Buy milk"}"#))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let todo: Todo = body_json(resp).await;
    assert_eq!(todo.title, "Buy milk");
    assert!(!todo.completed);
}

#[tokio::test]
async fn create_todo_with_completed_true() {
    let app = app();
    let resp = app
        .oneshot(json_request(
            "POST",
            "/todos",
            r#"{"title":"Already done","completed":true}"#,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let todo: Todo = body_json(resp).await;
    assert!(todo.completed);
}

#[tokio::test]
async fn create_todo_malformed_json_returns_422() {
    let app = app();
    let resp = app
        .oneshot(json_request("POST", "/todos", r#"{"not_title":1}"#))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// --- get ---

#[tokio::test]
async fn get_todo_not_found() {
    let app = app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/todos/00000000-0000-0000-0000-000000000000")
                .body(String::new())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_todo_bad_uuid_returns_400() {
    let app = app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/todos/not-a-uuid")
                .body(String::new())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- update ---

#[tokio::test]
async fn update_todo_not_found() {
    let app = app();
    let resp = app
        .oneshot(json_request(
            "PUT",
            "/todos/00000000-0000-0000-0000-000000000000",
            r#"{"title":"Nope"}"#,
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// --- delete ---

#[tokio::test]
async fn delete_todo_not_found() {
    let app = app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/todos/00000000-0000-0000-0000-000000000000")
                .body(String::new())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// --- full CRUD lifecycle ---

#[tokio::test]
async fn crud_lifecycle() {
    use tower::Service;

    let mut app = app().into_service();

    // create
    let resp = ServiceExt::ready(&mut app)
        .await
        .unwrap()
        .call(json_request("POST", "/todos", r#"{"title":"Walk dog"}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created: Todo = body_json(resp).await;
    assert_eq!(created.title, "Walk dog");
    assert!(!created.completed);
    let id = created.id;

    // list — should contain the one todo
    let resp = ServiceExt::ready(&mut app)
        .await
        .unwrap()
        .call(
            Request::builder()
                .uri("/todos")
                .body(String::new())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let todos: Vec<Todo> = body_json(resp).await;
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].id, id);

    // get
    let resp = ServiceExt::ready(&mut app)
        .await
        .unwrap()
        .call(
            Request::builder()
                .uri(&format!("/todos/{id}"))
                .body(String::new())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let fetched: Todo = body_json(resp).await;
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.title, "Walk dog");

    // update — partial: only completed
    let resp = ServiceExt::ready(&mut app)
        .await
        .unwrap()
        .call(json_request(
            "PUT",
            &format!("/todos/{id}"),
            r#"{"completed":true}"#,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let updated: Todo = body_json(resp).await;
    assert_eq!(updated.title, "Walk dog"); // unchanged
    assert!(updated.completed);

    // update — partial: only title
    let resp = ServiceExt::ready(&mut app)
        .await
        .unwrap()
        .call(json_request(
            "PUT",
            &format!("/todos/{id}"),
            r#"{"title":"Walk cat"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let updated: Todo = body_json(resp).await;
    assert_eq!(updated.title, "Walk cat");
    assert!(updated.completed); // unchanged from previous update

    // delete
    let resp = ServiceExt::ready(&mut app)
        .await
        .unwrap()
        .call(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/todos/{id}"))
                .body(String::new())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let body = body_bytes(resp).await;
    assert!(body.is_empty());

    // get after delete — 404
    let resp = ServiceExt::ready(&mut app)
        .await
        .unwrap()
        .call(
            Request::builder()
                .uri(&format!("/todos/{id}"))
                .body(String::new())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // list after delete — empty
    let resp = ServiceExt::ready(&mut app)
        .await
        .unwrap()
        .call(
            Request::builder()
                .uri("/todos")
                .body(String::new())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let todos: Vec<Todo> = body_json(resp).await;
    assert!(todos.is_empty());
}
