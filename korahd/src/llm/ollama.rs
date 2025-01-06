use crate::llm::{BoxLlm, Error, Llm, ToolCall};
use futures::future::BoxFuture;
use reqwest::{Client, Url};
use schemars::schema::RootSchema;
use serde::Serialize;

/// An Ollama LLM client.
pub struct Ollama {
    base_url: Url,
    client: Client,
}

#[derive(Serialize)]
struct PullRequestPayload {
    model: String,
    insecure: bool,
    stream: bool,
}

impl Ollama {
    /// Creates an Ollama instance in a form of BoxLlm.
    pub fn new_boxed(base_url: Url) -> Result<BoxLlm, Error> {
        if base_url.cannot_be_a_base() {
            return Err(Error::UnsupportedUrl);
        }
        Ok(Box::new(Self {
            base_url,
            client: Client::new(),
        }))
    }
}

impl Llm for Ollama {
    fn prepare_model(&self, model: &str) -> BoxFuture<Result<(), Error>> {
        let request = PullRequestPayload {
            model: model.to_owned(),
            insecure: false,
            stream: false,
        };

        let mut url = self.base_url.clone();
        url.set_path(&format!("{}api/pull", url.path()));

        let client = self.client.clone();

        Box::pin(async move {
            client
                .post(url)
                .json(&request)
                .send()
                .await?
                .error_for_status()?;

            Ok(())
        })
    }

    fn derive_tool_call(
        &self,
        _tools: &[RootSchema],
        _query: &str,
    ) -> BoxFuture<Result<Option<ToolCall>, Error>> {
        todo!()
    }
}
