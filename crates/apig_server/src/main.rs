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

    // disable temporal routes in prod/stage until auth is set up
    let versioned_api_router = match environment {
        Environments::stage | Environments::prod => versioned_api_router,
        _ => {
            // /api/:version/temporal
            let temporal_router = Router::new()
                .route("/", post(temporal_interaction_handler))
                .route("/encode", post(temporal_encoder))
                .route("/decode", post(temporal_decoder))
                .layer(TraceLayer::new_for_http());

            versioned_api_router.nest("/temporal", temporal_router)
        }
    };

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
            std::env::var("RUST_LOG").unwrap_or_else(|_| {
                "apig_server=trace,tower_http=trace,temporal_sdk_helpers=trace".into()
            }),
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
    use serde_json::json;
    use tower::ServiceExt; // for `oneshot` and `ready`

    async fn oneshot(
        method: &str,
        uri: &str,
        body: Body,
        assert_statuscode: StatusCode,
        body_is_json: bool,
    ) -> Bytes {
        let app = create_router(Environments::local).into_service();

        let mut request = Request::builder().uri(uri).method(method);
        if body_is_json {
            request = request.header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref());
        }

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
    async fn version_confidence_check() {
        let body = oneshot("GET", "/api/v1", Body::empty(), StatusCode::OK, false).await;
        assert_eq!(
            &String::from_utf8(body.to_vec()).unwrap(),
            "received request with version V1"
        )
    }

    #[tokio::test]
    async fn hello_world_invalid_version() {
        let body = oneshot(
            "GET",
            "/api/not-a-version",
            Body::empty(),
            StatusCode::NOT_FOUND,
            false,
        )
        .await;
        assert_eq!(&body[..], versions::UNSUPPORTED_API_VERSION_MSG.as_bytes())
    }

    #[tokio::test]
    async fn not_found_404() {
        let body = oneshot(
            "GET",
            "/does-not-exist",
            Body::empty(),
            StatusCode::NOT_FOUND,
            false,
        )
        .await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn invalid_request_to_temporal_route() {
        let body = oneshot(
            "POST",
            "/api/v1/temporal",
            Body::from(
                serde_json::to_vec(&json!({"not the right format" : "for temporal route"}))
                    .unwrap(),
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
            true,
        )
        .await;

        assert!(String::from_utf8_lossy(&body.to_vec())
            .contains("Failed to deserialize the JSON body into the target type:"))
    }

    // #[tokio::test]
    // async fn json() {
    //     let app = create_router().into_service();

    //     let response = app
    //         .oneshot(
    //             Request::builder()
    //                 .method(http::Method::POST)
    //                 .uri("/json")
    //                 .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
    //                 .body(Body::from(
    //                     serde_json::to_vec(&json!([1, 2, 3, 4])).unwrap(),
    //                 ))
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     assert_eq!(response.status(), StatusCode::OK);

    //     let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    //     let body: Value = serde_json::from_slice(&body).unwrap();
    //     assert_eq!(body, json!({ "data": [1, 2, 3, 4] }));
    // }

    #[tokio::test]
    async fn not_found() {
        let app = create_router(Environments::local).into_service();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert!(body.is_empty());
    }

    // You can also spawn a server and talk to it like any other HTTP server:
    // #[tokio::test]
    // async fn the_real_deal() {
    //     let listener = TcpListener::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap()).unwrap();
    //     let addr = listener.local_addr().unwrap();

    //     tokio::spawn(async move {
    //         axum::Server::from_tcp(listener)
    //             .unwrap()
    //             .serve(create_router().into_make_service())
    //             .await
    //             .unwrap();
    //     });

    //     let client = hyper::Client::new();

    //     let response = client
    //         .request(
    //             Request::builder()
    //                 .uri(format!("http://{}", addr))
    //                 .body(Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();

    //     let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    //     assert_eq!(&body[..], b"Hello, World!");
    // }

    // You can use `ready()` and `call()` to avoid using `clone()`
    // in multiple request
    // #[tokio::test]
    // async fn multiple_request() {
    //     let mut app = create_router().into_service();

    //     let request = Request::builder().uri("/").body(Body::empty()).unwrap();
    //     let response = app.ready().await.unwrap().call(request).await.unwrap();
    //     assert_eq!(response.status(), StatusCode::OK);

    //     let request = Request::builder().uri("/").body(Body::empty()).unwrap();
    //     let response = app.ready().await.unwrap().call(request).await.unwrap();
    //     assert_eq!(response.status(), StatusCode::OK);
    // }
}
