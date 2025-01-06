use crate::api::{ApiState, Error};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::WithRejection;
use serde::Deserialize;
use std::sync::Arc;

/// An API query request payload.
#[derive(Deserialize)]
pub struct RequestPayload {
    _query: String,
}

/// Handles API query requests.
#[axum::debug_handler]
pub async fn process_query(
    State(_state): State<Arc<ApiState>>,
    WithRejection(Json(_request), _): WithRejection<Json<RequestPayload>, Error>,
) -> Result<impl IntoResponse, Error> {
    Ok(StatusCode::OK)
}
