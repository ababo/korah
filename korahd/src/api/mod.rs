pub mod query;
pub mod tool;

use crate::{
    api::{
        query::process_query,
        tool::{call_tool, create_tools, ApiTools},
    },
    db::Db,
    llm::BoxLlm,
    util::fmt::ErrorChainDisplay,
};
use axum::{
    extract::rejection::{JsonRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use log::{debug, error};
use serde_json::json;
use std::sync::Arc;

/// An API error.
pub trait ApiError {
    /// Returns a corresponding HTTP status.
    fn status(&self) -> StatusCode;

    /// Returns a corresponding code.
    fn code(&self) -> &str;
}

/// A top-level API error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to serve request")]
    Axum(
        #[from]
        #[source]
        axum::Error,
    ),
    #[error("malformed JSON payload")]
    AxumJsonRejection(
        #[from]
        #[source]
        JsonRejection,
    ),
    #[error("malformed URL query")]
    AxumQueryRejection(
        #[from]
        #[source]
        QueryRejection,
    ),
    #[error(transparent)]
    Db(#[from] crate::db::Error),
    #[error(transparent)]
    Llm(#[from] crate::llm::Error),
    #[error("failed to (de)serialize JSON")]
    SerdeJson(
        #[from]
        #[source]
        serde_json::Error,
    ),
    #[error(transparent)]
    Tool(#[from] crate::tool::Error),
    #[error("tool '{0}' not found")]
    ToolNotFound(String),
}

impl ApiError for Error {
    fn status(&self) -> StatusCode {
        use Error::*;
        match &self {
            Axum(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AxumJsonRejection(_)
            | AxumQueryRejection(_)
            | Db(_)
            | SerdeJson(_)
            | ToolNotFound(_) => StatusCode::BAD_REQUEST,
            Llm(err) => err.status(),
            Tool(err) => err.status(),
        }
    }

    fn code(&self) -> &str {
        use Error::*;
        match &self {
            Axum(_) => "axum",
            AxumJsonRejection(_) => "axum_json_rejection",
            AxumQueryRejection(_) => "axum_query_rejection",
            Db(_) => "db",
            Llm(err) => err.code(),
            SerdeJson(_) => "serde_json",
            Tool(err) => err.code(),
            ToolNotFound(_) => "tool_not_found",
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = self.status();
        match status {
            StatusCode::INTERNAL_SERVER_ERROR | StatusCode::TOO_MANY_REQUESTS => {
                error!("failed to serve HTTP request: {}", ErrorChainDisplay(&self));
            }
            _ => {
                debug!("failed to serve HTTP request: {}", ErrorChainDisplay(&self));
            }
        }

        let response = json!({
            "error": {
                "code": self.code(),
                "message": self.to_string()
            }
        });
        (status, Json(response)).into_response()
    }
}

/// An internal state for API handlers.
pub struct ApiState {
    db: Db,
    llm: BoxLlm,
    tools: ApiTools,
}

/// Creates an Axum API router.
pub fn create_api(db: Db, llm: BoxLlm) -> Router {
    let tools = create_tools();
    let state = Arc::new(ApiState { db, llm, tools });
    Router::new()
        .route("/query", post(process_query))
        .route("/tool", post(call_tool))
        .with_state(state)
}
