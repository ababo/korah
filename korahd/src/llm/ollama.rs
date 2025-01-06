use crate::llm::{Error, Llm};
use reqwest::{Client, Url};
use serde::Serialize;

/// An Ollama LLM client.
pub struct Ollama {
    base_url: Url,
    client: Client,
}

#[derive(Serialize)]
struct PullRequestPayload<'a> {
    model: &'a str,
    insecure: bool,
    stream: bool,
}

impl Ollama {
    /// Creates an Ollama instance.
    pub fn new(base_url: Url) -> Result<Self, Error> {
        if base_url.cannot_be_a_base() {
            return Err(Error::UnsupportedUrl);
        }
        Ok(Self {
            base_url,
            client: Client::new(),
        })
    }
}

impl Llm for Ollama {
    async fn prepare_model(&self, model: &str) -> Result<(), Error> {
        let request = PullRequestPayload {
            model,
            insecure: false,
            stream: false,
        };

        let mut url = self.base_url.clone();
        url.set_path(&format!("{}api/pull", url.path()));

        self.client
            .post(url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
