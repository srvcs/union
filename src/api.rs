use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

/// This service's identity. `srvcs-union` is a leaf: it depends on no other
/// service. It computes the union of two sets of integers entirely with local
/// logic.
pub const SERVICE: &str = "srvcs-union";
pub const CONCERN: &str = "sets: union of two sets";
pub const DEPENDS_ON: &[&str] = &[];

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    /// The first set. Every element must be a JSON integer.
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    /// The second set. Every element must be a JSON integer.
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct UnionResponse {
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
    pub result: Vec<i64>,
}

/// The single concern: the union of sets `a` and `b`.
///
/// Returns `None` if any element of either list is not a JSON integer;
/// otherwise `Some` of the sorted list of distinct values appearing in `a` or
/// `b`.
pub fn union(a: &[Value], b: &[Value]) -> Option<Vec<i64>> {
    let mut set = std::collections::BTreeSet::new();
    for v in a.iter().chain(b.iter()) {
        match v.as_i64() {
            Some(n) => {
                set.insert(n);
            }
            None => return None,
        }
    }
    Some(set.into_iter().collect())
}

/// `POST /` — the union of sets `a` and `b`.
///
/// Reads each element of both lists as a JSON integer. If any element is not an
/// integer the request is rejected with `422`. Otherwise the sorted list of
/// distinct values appearing in `a` or `b` is returned as `result`.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = UnionResponse),
        (status = 422, description = "an element is not a valid integer")
    )
)]
pub async fn evaluate(Json(req): Json<EvalRequest>) -> Response {
    match union(&req.a, &req.b) {
        Some(result) => (
            StatusCode::OK,
            Json(json!({ "a": req.a, "b": req.b, "result": result })),
        )
            .into_response(),
        None => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "a and b must be integers" })),
        )
            .into_response(),
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, UnionResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some(), "GET / documented");
        assert!(root.post.is_some(), "POST / documented");
    }

    #[test]
    fn index_reports_identity() {
        // Identity constants are the public contract of this leaf service.
        assert_eq!(SERVICE, "srvcs-union");
        assert_eq!(CONCERN, "sets: union of two sets");
        assert!(DEPENDS_ON.is_empty());
    }

    #[test]
    fn union_of_two_sets() {
        assert_eq!(
            union(&[json!(1), json!(2)], &[json!(2), json!(3)]),
            Some(vec![1, 2, 3])
        );
    }

    #[test]
    fn distinct_and_sorted() {
        // Duplicates within and across the lists collapse; output is sorted.
        assert_eq!(
            union(
                &[json!(3), json!(3), json!(1)],
                &[json!(1), json!(2), json!(2)]
            ),
            Some(vec![1, 2, 3])
        );
    }

    #[test]
    fn handles_negatives_and_empty() {
        assert_eq!(
            union(&[json!(0), json!(-5)], &[json!(-5), json!(7)]),
            Some(vec![-5, 0, 7])
        );
        assert_eq!(union(&[], &[]), Some(vec![]));
        assert_eq!(union(&[json!(4)], &[]), Some(vec![4]));
        assert_eq!(union(&[], &[json!(4)]), Some(vec![4]));
    }

    #[test]
    fn non_integer_element_is_rejected() {
        for bad in [
            json!("1"),
            json!(1.5),
            json!(true),
            json!(null),
            json!([1]),
            json!({ "v": 1 }),
        ] {
            assert_eq!(
                union(&[json!(1)], &[bad.clone(), json!(2)]),
                None,
                "{bad} should be rejected in b"
            );
            assert_eq!(
                union(&[bad.clone(), json!(2)], &[json!(1)]),
                None,
                "{bad} should be rejected in a"
            );
        }
    }

    #[tokio::test]
    async fn evaluate_returns_200_with_result() {
        let resp = evaluate(Json(EvalRequest {
            a: vec![json!(1), json!(2)],
            b: vec![json!(2), json!(3)],
        }))
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn evaluate_returns_422_for_non_integer() {
        let resp = evaluate(Json(EvalRequest {
            a: vec![json!(1)],
            b: vec![json!(1.5)],
        }))
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn index_reports_identity_over_http() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-union");
        assert!(info.depends_on.is_empty());
    }
}
