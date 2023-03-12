use crate::{slack::axum_apig_handler_slack_interactions_api, versions::ApiVersion, AppError};
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use temporal_sdk_helpers::{execute_interaction, Encoder, TemporalInteraction};
use tower_http::{trace::TraceLayer, validate_request::ValidateRequestHeaderLayer};
use tracing::info;

/// `/api/:version/`
///
/// `/api/:version/slack/interaction`
///
/// `/api/:version/temporal/interact` (api-key protected)
///
/// `/api/:version/temporal/encode`
///
/// `/api/:version/temporal/decode`
pub fn create_router() -> Router {
    // keep slack routes separate so we can add Slack Verification layer, shared client, etc

    Router::new().nest(
        "/api/:version",
        Router::new()
            .route("/", get(version_confidence_check))
            .nest("/slack", create_slack_router())
            .nest("/temporal", create_temporal_router())
            .layer(TraceLayer::new_for_http()),
    )
}

fn create_slack_router() -> Router {
    Router::new().route(
        "/interaction",
        post(axum_apig_handler_slack_interactions_api),
    )
}

fn create_temporal_router() -> Router {
    Router::new()
        // Require the `Authorization` header to be `Bearer passwordlol`
        .route("/interact", post(temporal_interaction_handler))
        .layer(ValidateRequestHeaderLayer::bearer("passwordlol"))
        // routes below are not authenticated
        .route("/encode", post(temporal_encoder))
        .route("/decode", post(temporal_decoder))
}

// Route Handlers: ////////////////////////////////////////////////////////////

/// `/api/:version/`
///
/// fails if given an invalid ApiVersion
async fn version_confidence_check(api_version: ApiVersion) -> String {
    let message = format!("received request with version {:?}", api_version);
    info!(message);
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
