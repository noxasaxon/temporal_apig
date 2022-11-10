use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use std::collections::HashMap;

#[derive(Debug)]
pub enum ApiVersion {
    V1,
}

pub const UNSUPPORTED_API_VERSION_MSG: &str = "unsupported API version, route is invalid";

/// Convert Axum url request path for <version> into a supported API version
#[async_trait]
impl<S> FromRequestParts<S> for ApiVersion
where
    S: Send + Sync,
{
    type Rejection = Response;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let params: Path<HashMap<String, String>> =
            parts.extract().await.map_err(IntoResponse::into_response)?;
        let version = params
            .get("version")
            .ok_or_else(|| (StatusCode::NOT_FOUND, "version param missing").into_response())?;
        match version.as_str() {
            "v1" => Ok(ApiVersion::V1),
            _ => Err((StatusCode::NOT_FOUND, UNSUPPORTED_API_VERSION_MSG).into_response()),
        }
    }
}
