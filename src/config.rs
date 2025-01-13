use serde::Deserialize;
use strum::EnumString;
use url::Url;

#[derive(Clone, Copy, Debug, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum LlmApi {
    Ollama,
}

/// An Ollama LLM API configuration.
#[derive(Debug, Deserialize)]
pub struct OllamaConfig {
    pub base_url: Url,
    pub model: String,
}

/// An LLM API configuration.
#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub api: LlmApi,
    pub ollama: Option<OllamaConfig>,
    pub query_fmt: String,
}

/// A program configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
}
