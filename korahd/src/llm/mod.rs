pub mod context;
pub mod ollama;

use crate::api::{tool::ToolMetadata, ApiError};
use futures::future::BoxFuture;
use reqwest::StatusCode;
use serde_json::value::RawValue;

/// An LLM error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("reqwest")]
    Reqwest(
        #[from]
        #[source]
        reqwest::Error,
    ),
    #[error("unsupported url")]
    UnsupportedUrl,
}

impl ApiError for Error {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn code(&self) -> &str {
        use Error::*;
        match self {
            Reqwest(_) => "llm_reqwest",
            UnsupportedUrl => "llm_unsupported_url",
        }
    }
}

/// A tool call derived by LLM.
#[derive(Debug)]
pub struct ToolCall {
    pub name: String,
    pub params: Box<RawValue>,
}

/// An LLM API client.
pub trait Llm {
    /// Makes LLM server prepare a model for a subsequent use.
    fn prepare_model(&self, model: &str) -> BoxFuture<Result<(), Error>> {
        _ = model; // Avoid 'unused' warning.
        Box::pin(async { Ok(()) })
    }

    /// Derives a tool call from a given query.
    fn derive_tool_call(
        &self,
        model: String,
        tools: Vec<ToolMetadata>,
        query: String,
    ) -> BoxFuture<Result<Option<ToolCall>, Error>>;
}

/// An owned dynamically typed Llm.
pub type BoxLlm = Box<dyn Llm + Send + Sync>;
