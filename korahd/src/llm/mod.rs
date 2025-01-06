pub mod ollama;

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

/// An LLM API client.
pub trait Llm {
    /// Makes LLM server prepare a model for a subsequent use.
    async fn prepare_model(&self, model: &str) -> Result<(), Error> {
        _ = model; // Avoid 'unused' warning.
        Ok(())
    }
}
