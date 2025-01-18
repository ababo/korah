use crate::llm::LlmConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};

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
    pub double_pass_derive: bool,
    pub llm: LlmConfig,
    pub num_derive_tries: u32,
}

impl Config {
    /// A common basename of the configuration file.
    pub const COMMON_FILE_BASENAME: &str = "korah.toml";

    /// Searches for the configuration file in common directories and returns its path if found.
    pub fn find_common_path() -> Option<PathBuf> {
        #[cfg(unix)]
        let paths = vec![".", "$HOME/.config", "/etc"];

        #[cfg(windows)]
        let paths = vec![".", "$USERPROFILE", "$SystemDrive"];

        for path in paths {
            let filename = PathBuf::from(path).join(Self::COMMON_FILE_BASENAME);
            let filename = shellexpand::path::env(&filename).unwrap();
            if filename.exists() {
                return Some(filename.to_path_buf());
            }
        }

        None
    }

    /// Reads program configuration from a file.
    pub fn read(path: &Path) -> Result<Self, Error> {
        let s = std::fs::read_to_string(path)?;
        toml::from_str(&s).map_err(Into::into)
    }
}
