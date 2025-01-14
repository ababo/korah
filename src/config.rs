use crate::llm::LlmConfig;
use serde::Deserialize;
use std::path::Path;

/// A program configuration error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to perform io")]
    SerdeJson(
        #[from]
        #[source]
        std::io::Error,
    ),
    #[error("failed to deserialize toml")]
    TomlDe(
        #[from]
        #[source]
        toml::de::Error,
    ),
}

/// A program configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub num_derive_tries: u32,
}

impl Config {
    /// Reads program configuration from a file.
    pub fn read(path: &Path) -> Result<Self, Error> {
        let s = std::fs::read_to_string(path)?;
        toml::from_str(&s).map_err(Into::into)
    }
}
