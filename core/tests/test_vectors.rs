//! Verify build/parse methods against JSON test vectors stored in `test-vectors/`.
//!
//! Each vector file describes inputs, expected requests, simulated responses,
//! and expected parse results. Comparing parsed JSON (not raw strings) avoids
//! false negatives from field-ordering differences.

use todo_core::{ApiError, CreateTodo, HttpMethod, HttpResponse, Todo, TodoClient, UpdateTodo};
use uuid::Uuid;

const BASE_URL: &str = "http://localhost:3000";

fn client() -> TodoClient {
    TodoClient::new(BASE_URL)
}

/// Parse the method string from test vectors into `HttpMethod`.
fn parse_method(s: &str) -> HttpMethod {
    match s {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "DELETE" => HttpMethod::Delete,
        other => panic!("unknown method: {other}"),
    }
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

#[test]
fn create_test_vectors() {
    let raw = include_str!("../../test-vectors/create.json");
    let vectors: serde_json::Value = serde_json::from_str(raw).unwrap();

    let c = client();
    for case in vectors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let input: CreateTodo = serde_json::from_value(case["input"].clone()).unwrap();
        let expected_req = &case["expected_request"];

        // Verify build
        let req = c.build_create_todo(&input).unwrap();
        assert_eq!(req.method, parse_method(expected_req["method"].as_str().unwrap()), "{name}: method");
        assert_eq!(req.path, format!("{BASE_URL}{}", expected_req["path"].as_str().unwrap()), "{name}: path");

        let expected_headers: Vec<(String, String)> = expected_req["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| {
                let arr = h.as_array().unwrap();
                (arr[0].as_str().unwrap().to_string(), arr[1].as_str().unwrap().to_string())
            })
            .collect();
        assert_eq!(req.headers, expected_headers, "{name}: headers");

        let req_body: serde_json::Value = serde_json::from_str(req.body.as_deref().unwrap()).unwrap();
        assert_eq!(req_body, expected_req["body"], "{name}: body");

        // Verify parse
        let sim = &case["simulated_response"];
        let response = HttpResponse {
            status: sim["status"].as_u64().unwrap() as u16,
            headers: Vec::new(),
            body: sim["body"].as_str().unwrap().to_string(),
        };
        let todo = c.parse_create_todo(response).unwrap();
        let expected: Todo = serde_json::from_value(case["expected_result"].clone()).unwrap();
        assert_eq!(todo, expected, "{name}: parsed result");
    }
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

#[test]
fn list_test_vectors() {
    let raw = include_str!("../../test-vectors/list.json");
    let vectors: serde_json::Value = serde_json::from_str(raw).unwrap();

    let c = client();
    for case in vectors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let expected_req = &case["expected_request"];

        // Verify build
        let req = c.build_list_todos();
        assert_eq!(req.method, parse_method(expected_req["method"].as_str().unwrap()), "{name}: method");
        assert_eq!(req.path, format!("{BASE_URL}{}", expected_req["path"].as_str().unwrap()), "{name}: path");
        assert!(req.body.is_none(), "{name}: body should be None");

        // Verify parse
        let sim = &case["simulated_response"];
        let response = HttpResponse {
            status: sim["status"].as_u64().unwrap() as u16,
            headers: Vec::new(),
            body: sim["body"].as_str().unwrap().to_string(),
        };
        let todos = c.parse_list_todos(response).unwrap();
        let expected: Vec<Todo> = serde_json::from_value(case["expected_result"].clone()).unwrap();
        assert_eq!(todos, expected, "{name}: parsed result");
    }
}

// ---------------------------------------------------------------------------
// Get
// ---------------------------------------------------------------------------

#[test]
fn get_test_vectors() {
    let raw = include_str!("../../test-vectors/get.json");
    let vectors: serde_json::Value = serde_json::from_str(raw).unwrap();

    let c = client();
    for case in vectors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let id: Uuid = case["input_id"].as_str().unwrap().parse().unwrap();
        let expected_req = &case["expected_request"];

        // Verify build
        let req = c.build_get_todo(id);
        assert_eq!(req.method, parse_method(expected_req["method"].as_str().unwrap()), "{name}: method");
        assert_eq!(req.path, format!("{BASE_URL}{}", expected_req["path"].as_str().unwrap()), "{name}: path");
        assert!(req.body.is_none(), "{name}: body should be None");

        // Verify parse
        let sim = &case["simulated_response"];
        let response = HttpResponse {
            status: sim["status"].as_u64().unwrap() as u16,
            headers: Vec::new(),
            body: sim["body"].as_str().unwrap().to_string(),
        };
        let result = c.parse_get_todo(response);

        if let Some(expected_error) = case.get("expected_error") {
            let err = result.unwrap_err();
            match expected_error.as_str().unwrap() {
                "NotFound" => assert!(matches!(err, ApiError::NotFound), "{name}: expected NotFound"),
                other => panic!("{name}: unknown expected_error: {other}"),
            }
        } else {
            let todo = result.unwrap();
            let expected: Todo = serde_json::from_value(case["expected_result"].clone()).unwrap();
            assert_eq!(todo, expected, "{name}: parsed result");
        }
    }
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

#[test]
fn update_test_vectors() {
    let raw = include_str!("../../test-vectors/update.json");
    let vectors: serde_json::Value = serde_json::from_str(raw).unwrap();

    let c = client();
    for case in vectors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let id: Uuid = case["input_id"].as_str().unwrap().parse().unwrap();
        let input: UpdateTodo = serde_json::from_value(case["input"].clone()).unwrap();
        let expected_req = &case["expected_request"];

        // Verify build
        let req = c.build_update_todo(id, &input).unwrap();
        assert_eq!(req.method, parse_method(expected_req["method"].as_str().unwrap()), "{name}: method");
        assert_eq!(req.path, format!("{BASE_URL}{}", expected_req["path"].as_str().unwrap()), "{name}: path");

        let req_body: serde_json::Value = serde_json::from_str(req.body.as_deref().unwrap()).unwrap();
        assert_eq!(req_body, expected_req["body"], "{name}: body");

        // Verify parse
        let sim = &case["simulated_response"];
        let response = HttpResponse {
            status: sim["status"].as_u64().unwrap() as u16,
            headers: Vec::new(),
            body: sim["body"].as_str().unwrap().to_string(),
        };
        let todo = c.parse_update_todo(response).unwrap();
        let expected: Todo = serde_json::from_value(case["expected_result"].clone()).unwrap();
        assert_eq!(todo, expected, "{name}: parsed result");
    }
}

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

#[test]
fn delete_test_vectors() {
    let raw = include_str!("../../test-vectors/delete.json");
    let vectors: serde_json::Value = serde_json::from_str(raw).unwrap();

    let c = client();
    for case in vectors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let id: Uuid = case["input_id"].as_str().unwrap().parse().unwrap();
        let expected_req = &case["expected_request"];

        // Verify build
        let req = c.build_delete_todo(id);
        assert_eq!(req.method, parse_method(expected_req["method"].as_str().unwrap()), "{name}: method");
        assert_eq!(req.path, format!("{BASE_URL}{}", expected_req["path"].as_str().unwrap()), "{name}: path");
        assert!(req.body.is_none(), "{name}: body should be None");

        // Verify parse
        let sim = &case["simulated_response"];
        let response = HttpResponse {
            status: sim["status"].as_u64().unwrap() as u16,
            headers: Vec::new(),
            body: sim["body"].as_str().unwrap().to_string(),
        };
        let result = c.parse_delete_todo(response);

        if let Some(expected_error) = case.get("expected_error") {
            let err = result.unwrap_err();
            match expected_error.as_str().unwrap() {
                "NotFound" => assert!(matches!(err, ApiError::NotFound), "{name}: expected NotFound"),
                other => panic!("{name}: unknown expected_error: {other}"),
            }
        } else {
            assert!(result.is_ok(), "{name}: expected success");
        }
    }
}
