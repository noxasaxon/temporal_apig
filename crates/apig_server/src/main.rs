mod config;
mod slack;
mod versions;

use crate::config::{init_config_from_env_and_file, Environments};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use slack::axum_apig_handler_slack_interactions_api;
use std::net::SocketAddr;
use temporal_sdk_helpers::{
    execute_interaction, Encoder, TemporalInteraction, TEMPORAL_HOST_PORT_PAIR,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use versions::ApiVersion;

fn create_router(environment: Environments) -> Router {
    // keep slack routes separate so we can add Slack Verification layer, shared client, etc
    // /api/:version/slack
    let slack_router = Router::new()
        .route(
            "/interaction",
            post(axum_apig_handler_slack_interactions_api),
        )
        .layer(TraceLayer::new_for_http());

    // /api/:version
    let versioned_api_router = Router::new()
        .route("/", get(version_confidence_check))
        .nest("/slack", slack_router);

    // /api/:version/temporal
    let temporal_router = Router::new()
        .route("/encode", post(temporal_encoder))
        .route("/decode", post(temporal_decoder));

    // disable non-slack event processing routes in prod/stage until api auth is set up
    let temporal_router = match environment {
        Environments::stage | Environments::prod => temporal_router,
        _ => temporal_router.route("/", post(temporal_interaction_handler)),
    }
    .layer(TraceLayer::new_for_http());

    let versioned_api_router = versioned_api_router.nest("/temporal", temporal_router);

    Router::new().nest("/api/:version", versioned_api_router)
}

#[tokio::main]
async fn main() {
    let config = init_config_from_env_and_file().expect("unable to build app config");

    TEMPORAL_HOST_PORT_PAIR
        .set((config.temporal_service_host, config.temporal_service_port))
        .expect("shouldn't fail");

    // TODO: add temporal cluster connection check before starting the webserver

    init_tracing();

    // build our application with versioned routes
    let app = create_router(config.environment);
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

// Route Handlers: ////////////////////////////////////////////////////////////

async fn version_confidence_check(api_version: ApiVersion) -> String {
    let message = format!("received request with version {:?}", api_version);
    println!("{}", &message);
    message
}

async fn temporal_encoder(
    api_version: ApiVersion,
    Json(payload): Json<TemporalInteraction>,
) -> Result<impl IntoResponse, AppError> {
    match api_version {
        ApiVersion::V1 => {
            let encoded_string = Encoder::default().encode(payload);
            Ok((StatusCode::CREATED, encoded_string))
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TemporalDecoderInput {
    encoded: String,
}

async fn temporal_decoder(
    api_version: ApiVersion,
    Json(payload): Json<TemporalDecoderInput>,
) -> Result<impl IntoResponse, AppError> {
    match api_version {
        ApiVersion::V1 => {
            let temporal_interaction = Encoder::decode(&payload.encoded)?;
            let as_string = serde_json::to_string(&temporal_interaction)?;

            Ok((StatusCode::CREATED, as_string))
        }
    }
}

async fn temporal_interaction_handler(
    api_version: ApiVersion,
    Json(payload): Json<TemporalInteraction>,
) -> Result<impl IntoResponse, AppError> {
    match api_version {
        ApiVersion::V1 => {
            let temporal_response = execute_interaction(payload).await?;
            Ok((StatusCode::CREATED, Json(temporal_response)))
        }
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
    use mime;
    use serde_json::json;
    use tower::ServiceExt; // for `oneshot` and `ready`

    async fn oneshot(
        method: &str,
        uri: &str,
        body: Body,
        assert_statuscode: StatusCode,
        mime_type: mime::Mime,
    ) -> Bytes {
        let app = create_router(Environments::local).into_service();

        let request = Request::builder()
            .uri(uri)
            .method(method)
            .header(http::header::CONTENT_TYPE, mime_type.as_ref());

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
        let body = oneshot(
            "GET",
            "/api/v1",
            Body::empty(),
            StatusCode::OK,
            mime::TEXT_PLAIN,
        )
        .await;
        assert_eq!(
            &String::from_utf8(body.to_vec()).unwrap(),
            "received request with version V1"
        )
    }

    #[tokio::test]
    async fn test_invalid_version() {
        let body = oneshot(
            "GET",
            "/api/not-a-version",
            Body::empty(),
            StatusCode::NOT_FOUND,
            mime::TEXT_PLAIN,
        )
        .await;
        assert_eq!(&body[..], versions::UNSUPPORTED_API_VERSION_MSG.as_bytes())
    }

    #[tokio::test]
    async fn test_route_not_found_404() {
        let body = oneshot(
            "GET",
            "/does-not-exist",
            Body::empty(),
            StatusCode::NOT_FOUND,
            mime::TEXT_PLAIN,
        )
        .await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn test_wrong_structure_sent_to_temporal_route() {
        let body = oneshot(
            "POST",
            "/api/v1/temporal",
            Body::from(
                serde_json::to_vec(&json!({"not the right format" : "for temporal route"}))
                    .unwrap(),
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
            mime::APPLICATION_JSON,
        )
        .await;

        assert!(String::from_utf8_lossy(&body.to_vec())
            .contains("Failed to deserialize the JSON body into the target type:"))
    }

    #[tokio::test]
    async fn test_encode_endpoint_v1_signal() {
        let signal_temporal_json = json!({
            "type" : "Signal",
            "namespace" : "my-namespace",
            "task_queue": "my-taskqueue",
            "run_id": "some-run-id",
            "workflow_id":"some-workflow-id",
            "signal_name": "my_signal_name"
        });

        let body = oneshot(
            "POST",
            "/api/v1/temporal/encode",
            Body::from(serde_json::to_vec(&signal_temporal_json).unwrap()),
            StatusCode::CREATED,
            mime::APPLICATION_JSON,
        )
        .await;

        assert_eq!("A~E:Signal,W:some-workflow-id,N:my-namespace,T:my-taskqueue,R:some-run-id,S:my_signal_name", body);
    }
}
