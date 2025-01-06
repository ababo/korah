pub mod tool;

use crate::{
    api::tool::{call_tool, create_tools, ApiTools},
    db::Db,
    llm::ollama::Ollama,
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

impl Error {
    /// Returns a corresponding HTTP status.
    pub fn status(&self) -> StatusCode {
        use Error::*;
        match &self {
            Axum(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AxumJsonRejection(_) | AxumQueryRejection(_) | SerdeJson(_) | ToolNotFound(_) => {
                StatusCode::BAD_REQUEST
            }
            Tool(err) => err.status(),
        }
    }

    /// Returns a corresponding code.
    pub fn code(&self) -> &str {
        use Error::*;
        match &self {
            Axum(_) => "axum",
            AxumJsonRejection(_) => "axum_json_rejection",
            AxumQueryRejection(_) => "axum_query_rejection",
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
    _db: Db,
    _ollama: Ollama,
    tools: ApiTools,
}

/// Creates an Axum API router.
pub fn create_api(db: Db, ollama: Ollama) -> Router {
    let tools = create_tools();
    let state = Arc::new(ApiState {
        _db: db,
        _ollama: ollama,
        tools,
    });
    Router::new()
        .route("/tool", post(call_tool))
        .with_state(state)
}
