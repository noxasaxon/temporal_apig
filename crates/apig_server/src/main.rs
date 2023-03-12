mod config;
mod routes;
mod slack;
mod versions;

use crate::{config::init_config_from_env_and_file, routes::create_router};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use temporal_sdk_helpers::TEMPORAL_HOST_PORT_PAIR;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let config = init_config_from_env_and_file().expect("unable to build app config");

    TEMPORAL_HOST_PORT_PAIR
        .set((config.temporal_service_host, config.temporal_service_port))
        .expect("shouldn't fail");

    // TODO: add temporal cluster connection check before starting the webserver

    init_tracing();

    // build our application with versioned routes
    let app = create_router();
    // run it
    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        config.apig_port.parse().expect("not a valid port"),
    ));
    tracing::debug!("listening on {}", addr);

    if let Err(err) = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
    {
        tracing::error!("server error: {}", err);
        eprintln!("server error: {}", err);
    }
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| 
                // "apig_server=debug".into()
                "apig_server=trace,tower_http=trace,temporal_sdk_helpers=trace".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

// TESTS ---------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, Bytes},
        http::{self, Request, StatusCode},
    };
    use hyper::header::AUTHORIZATION;
    use mime;
    use serde_json::json;
    use temporal_sdk_helpers::TemporalInteraction;
    use tower::ServiceExt; // for `oneshot` and `ready`

    async fn oneshot(
        request: axum::http::request::Builder,
        body: Body,
        assert_statuscode: StatusCode,
    ) -> Bytes {
        let app = create_router();

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = app
            .oneshot(request.body(body).expect("request body is invalid"))
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            assert_statuscode,
            "response's status code is not what we expected"
        );

        hyper::body::to_bytes(response.into_body())
            .await
            .expect("unable to convert response body to bytes")
    }

    #[tokio::test]
    async fn test_versioning_exists() {
        let request = Request::builder()
            .uri("/api/v1")
            .method("GET")
            .header(http::header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref());

        let body = oneshot(request, Body::empty(), StatusCode::OK).await;
        assert_eq!(
            &String::from_utf8(body.to_vec()).unwrap(),
            "received request with version V1"
        )
    }

    #[tokio::test]
    async fn test_invalid_version() {
        let request = Request::builder()
            .uri("/api/not-a-version")
            .method("GET")
            .header(http::header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref());

        let body = oneshot(request, Body::empty(), StatusCode::NOT_FOUND).await;
        assert_eq!(&body[..], versions::UNSUPPORTED_API_VERSION_MSG.as_bytes())
    }

    #[tokio::test]
    async fn test_route_not_found_404() {
        let request = Request::builder()
            .uri("/does-not-exist")
            .method("GET")
            .header(http::header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref());

        let body = oneshot(request, Body::empty(), StatusCode::NOT_FOUND).await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn test_wrong_structure_sent_to_temporal_route() {
        let request = Request::builder()
            .uri("/api/v1/temporal/interact")
            .method("POST")
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .header(AUTHORIZATION, &format!("Bearer {}", "passwordlol"));

        let body = oneshot(
            request,
            Body::from(
                serde_json::to_vec(&json!({"not the right format" : "for temporal route"}))
                    .unwrap(),
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;

        assert!(String::from_utf8_lossy(&body.to_vec())
            .contains("Failed to deserialize the JSON body into the target type:"))
    }

    #[tokio::test]
    async fn test_encode_endpoint_v1_signal() {
        let request = Request::builder()
            .uri("/api/v1/temporal/encode")
            .method("POST")
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref());

        let signal_temporal_json = json!({
            "type" : "Signal",
            "namespace" : "my-namespace",
            "task_queue": "my-taskqueue",
            "run_id": "some-run-id",
            "workflow_id":"some-workflow-id",
            "signal_name": "my_signal_name"
        });

        let body = oneshot(
            request,
            Body::from(serde_json::to_vec(&signal_temporal_json).unwrap()),
            StatusCode::CREATED,
        )
        .await;

        assert_eq!("A~E:Signal,W:some-workflow-id,N:my-namespace,T:my-taskqueue,R:some-run-id,S:my_signal_name", body);
    }

    #[tokio::test]
    async fn test_encode_and_decode_endpoints_v1_signal() {
        let signal_temporal_json = json!({
            "type" : "Signal",
            "namespace" : "my-namespace",
            "task_queue": "my-taskqueue",
            "run_id": "some-run-id",
            "workflow_id":"some-workflow-id",
            "signal_name": "my_signal_name"
        });

        let request = Request::builder()
            .uri("/api/v1/temporal/encode")
            .method("POST")
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref());

        let body = oneshot(
            request,
            Body::from(serde_json::to_vec(&signal_temporal_json).unwrap()),
            StatusCode::CREATED,
        )
        .await;

        let encoded_string_result = "A~E:Signal,W:some-workflow-id,N:my-namespace,T:my-taskqueue,R:some-run-id,S:my_signal_name";
        assert_eq!(encoded_string_result, body);

        let request = Request::builder()
            .uri("/api/v1/temporal/decode")
            .method("POST")
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref());

        let body = oneshot(
            request,
            Body::from(serde_json::to_vec(&json!({ "encoded": encoded_string_result })).unwrap()),
            StatusCode::CREATED,
        )
        .await;

        assert_eq!(
            serde_json::from_value::<TemporalInteraction>(signal_temporal_json).unwrap(),
            serde_json::from_slice::<TemporalInteraction>(&body[..]).unwrap()
        );
    }
}
