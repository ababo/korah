pub mod ollama;

#[derive(thiserror::Error, Debug)]
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

pub trait Llm {
    async fn prepare_model(&self, model: &str) -> Result<(), Error>;
}
