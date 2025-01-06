pub mod find_files;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;

/// A tool error.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl Error {
    /// Returns a corresponding HTTP status.
    pub fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    /// Returns a corresponding code.
    pub fn code(&self) -> &str {
        "bad request"
    }
}

/// Tool call parameters.
#[derive(Deserialize)]
pub struct Params<P> {
    #[serde(flatten)]
    tool_specific: P,
}

/// Tool-generated event.
#[derive(Debug, Serialize)]
pub struct Event<E> {
    #[serde(flatten)]
    tool_specific: E,
}

/// Generic tool.
pub trait Tool {
    /// A tool-specific parameters.
    type Params;

    /// A tool-specific event.
    type Event;

    /// Calls the tool with given parameters getting an event stream.
    fn call(
        &self,
        params: Params<Self::Params>,
    ) -> Result<UnboundedReceiver<Event<Self::Event>>, Error>;
}
