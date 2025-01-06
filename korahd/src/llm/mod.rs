pub mod ollama;

use futures::future::BoxFuture;
use schemars::schema::RootSchema;
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

/// A tool call derived by LLM.
pub struct ToolCall {
    pub _name: String,
    pub _params: Box<RawValue>,
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
        tools: &[RootSchema],
        query: &str,
    ) -> BoxFuture<Result<Option<ToolCall>, Error>>;
}

/// An owned dynamically typed Llm.
pub type BoxLlm = Box<dyn Llm + Send + Sync>;
