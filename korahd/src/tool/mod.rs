pub mod find_files;

use crate::api::ApiError;
use reqwest::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;

/// A tool error.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl ApiError for Error {
    fn status(&self) -> StatusCode {
        unreachable!();
    }

    fn code(&self) -> &str {
        unreachable!();
    }
}

/// Tool call parameters.
#[derive(Deserialize, JsonSchema)]
pub struct Params<P> {
    #[serde(flatten)]
    _tool_specific: P,
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

    /// An optional tool description.
    fn description(&self) -> Option<&'static str> {
        None
    }

    /// A tool name.
    fn name(&self) -> &'static str;
}
