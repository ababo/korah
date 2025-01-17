pub mod ollama;
pub mod open_ai;

use crate::{
    llm::{
        ollama::{OllamaClient, OllamaConfig},
        open_ai::{OpenAiClient, OpenAiConfig},
    },
    tool::ToolMeta,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use strfmt::strfmt;
use sys_locale::get_locale;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmApi {
    Ollama,
    OpenAi,
}

/// An LLM API configuration.
#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub api: LlmApi,
    pub ollama: Option<OllamaConfig>,
    pub open_ai: Option<OpenAiConfig>,
    pub query_fmt: String,
}

/// An LLM API error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
    #[error("malformed config: {0}")]
    MalformedConfig(&'static str),
    #[error("failed to (de)serialize json")]
    SerdeJson(
        #[from]
        #[source]
        serde_json::Error,
    ),
    #[error("failed to expand environment variables")]
    Shellexpend(
        #[from]
        #[source]
        shellexpand::LookupError<std::env::VarError>,
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
#[derive(Debug, Deserialize, Serialize)]
pub struct ToolCall {
    pub tool: String,
    pub params: Box<RawValue>,
}

/// An LLM API client.
pub trait LlmClient {
    /// Derives a tool call from a given query.
    fn derive_tool_call(
        &self,
        tools: Vec<ToolMeta>,
        query: String,
    ) -> Result<Option<ToolCall>, Error>;
}

/// An owned dynamically typed LLM API client.
pub type BoxLlm = Box<dyn LlmClient>;

/// Creates an LLM API client.
pub fn create_llm_client(config: &LlmConfig) -> Result<BoxLlm, Error> {
    use LlmApi::*;
    Ok(match config.api {
        Ollama => {
            let Some(config) = &config.ollama else {
                return Err(Error::MalformedConfig("missing ollama config"));
            };
            OllamaClient::new_boxed(config.clone())
        }
        OpenAi => {
            let Some(config) = &config.open_ai else {
                return Err(Error::MalformedConfig("missing open ai config"));
            };
            OpenAiClient::new_boxed(config.clone())
        }
    })
}

/// An LLM query context.
#[derive(Serialize)]
pub struct Context {
    os_name: &'static str,
    system_locale: String,
    time_now: DateTime<Utc>,
    username: String,
}

impl Context {
    /// Creates a default Context instance.
    pub fn new() -> Context {
        Context {
            os_name: std::env::consts::OS,
            system_locale: get_locale().unwrap_or("en-US".to_owned()),
            time_now: Utc::now(),
            username: whoami::username(),
        }
    }

    /// Contextualizes a given LLM query.
    pub fn contextualize(&self, config: &LlmConfig, query: String) -> String {
        let context = serde_json::to_string(&Self::new()).unwrap();

        let mut vars = HashMap::new();
        vars.insert("context".to_owned(), context);
        vars.insert("query".to_owned(), query);

        strfmt(&config.query_fmt, &vars).unwrap()
    }
}
