use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_union::{health, router, telemetry};
use tower::ServiceExt;

fn app() -> axum::Router {
    router(telemetry::metrics_handle_for_tests())
}

async fn status_of(uri: &str) -> StatusCode {
    app()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

/// POST `{ "a": <a>, "b": <b> }` to `/` and return (status, parsed JSON).
async fn eval(a: Value, b: Value) -> (StatusCode, Value) {
    let res = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "a": a, "b": b }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

// --- Standard srvcs service surface ---

#[tokio::test]
async fn index_ok() {
    assert_eq!(status_of("/").await, StatusCode::OK);
}

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

// --- Union cases ---

#[tokio::test]
async fn union_of_two_sets() {
    let (status, body) = eval(json!([1, 2]), json!([2, 3])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([1, 2, 3]));
    assert_eq!(body["a"], json!([1, 2]));
    assert_eq!(body["b"], json!([2, 3]));
}

#[tokio::test]
async fn duplicates_collapse_and_output_is_sorted() {
    let (status, body) = eval(json!([3, 3, 1]), json!([1, 2, 2])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([1, 2, 3]));
}

#[tokio::test]
async fn unsorted_inputs_yield_sorted_distinct() {
    let (status, body) = eval(json!([5, 0, 5]), json!([3, 0, -2])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([-2, 0, 3, 5]));
}

#[tokio::test]
async fn empty_lists_yield_empty() {
    let (status, body) = eval(json!([]), json!([])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([]));
}

#[tokio::test]
async fn one_empty_list_yields_the_other_distinct() {
    let (status, body) = eval(json!([4, 4, 1]), json!([])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([1, 4]));

    let (status, body) = eval(json!([]), json!([7, 7])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([7]));
}

// --- Error / edge cases ---

#[tokio::test]
async fn non_integer_element_in_b_is_422() {
    let (status, body) = eval(json!([1, 2]), json!([2, "nope"])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "a and b must be integers");
}

#[tokio::test]
async fn float_element_in_a_is_422() {
    let (status, body) = eval(json!([1, 1.5]), json!([2])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "a and b must be integers");
}

#[tokio::test]
async fn missing_field_is_422() {
    // A body missing the `b` field is a client error, not a 500.
    let res = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "a": [1] }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}
