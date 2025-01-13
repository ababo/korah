pub mod ollama;

use serde_json::value::RawValue;

/// An LLM API error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
    #[error("ureq error")]
    Ureq(
        #[from]
        #[source]
        Box<ureq::Error>,
    ),
}

impl From<ureq::Error> for Error {
    fn from(value: ureq::Error) -> Self {
        Error::Ureq(Box::new(value))
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
    /// Derives a tool call from a given query.
    fn derive_tool_call(&self, query: &str) -> Result<Option<ToolCall>, Error>;
}

/// An owned dynamically typed LLM API client.
pub type BoxLlm = Box<dyn Llm>;
