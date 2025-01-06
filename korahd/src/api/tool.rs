use crate::{
    api::{ApiState, Error},
    tool::{find_files::FindFiles, Tool},
    util::fmt::ErrorChainDisplay,
};
use axum::{
    extract::State,
    response::{sse::Event as SseEvent, IntoResponse, Sse},
    Json,
};
use axum_extra::extract::WithRejection;
use futures::{stream::BoxStream, StreamExt};
use log::warn;
use schemars::{schema::RootSchema, schema_for, JsonSchema};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use tokio_stream::wrappers::UnboundedReceiverStream;

/// A tool wrapper for API dynamic dispatch.
pub trait ApiTool {
    /// Calls the tool with given parameters getting an event stream.
    fn api_call(
        self: Arc<Self>,
        params: Box<RawValue>,
    ) -> Result<BoxStream<'static, Box<RawValue>>, Error>;

    /// Returns the tool metadata.
    fn metadata(&self) -> RootSchema;
}

impl<T> ApiTool for T
where
    T: Tool,
    T::Params: DeserializeOwned + JsonSchema,
    T::Event: Debug + Send + Serialize + 'static,
{
    fn api_call(
        self: Arc<Self>,
        params: Box<RawValue>,
    ) -> Result<BoxStream<'static, Box<RawValue>>, Error> {
        let params = serde_json::from_str(params.get())?;
        let events = self.call(params)?;
        let events = UnboundedReceiverStream::new(events);
        let events = events.filter_map(|e| async move {
            match serde_json::to_string(&e).and_then(RawValue::from_string) {
                Ok(event) => Some(event),
                Err(err) => {
                    warn!(
                        "failed to serialize tool event {e:?}: {}",
                        ErrorChainDisplay(&err)
                    );
                    None
                }
            }
        });
        Ok(events.boxed())
    }

    fn metadata(&self) -> RootSchema {
        // The parameters' schema title and description are
        // used as the tool's name and description respectively.
        schema_for!(T::Params)
    }
}

/// A mapping from tool names to their corresponding tool instances.
pub type ApiTools = HashMap<&'static str, Arc<dyn ApiTool + Send + Sync>>;

/// Creates API tools.
pub fn create_tools() -> ApiTools {
    let mut tools = ApiTools::new();
    tools.insert("find_files", Arc::new(FindFiles::new()));
    tools
}

/// An API tool call request payload.
#[derive(Deserialize)]
pub struct RequestPayload {
    tool: String,
    params: Box<RawValue>,
}

/// Handles API tool call requests.
#[axum::debug_handler]
pub async fn call_tool(
    State(state): State<Arc<ApiState>>,
    WithRejection(Json(request), _): WithRejection<Json<RequestPayload>, Error>,
) -> Result<impl IntoResponse, Error> {
    let Some(tool) = state.tools.get(request.tool.as_str()) else {
        return Err(Error::ToolNotFound(request.tool));
    };

    let events = tool.clone().api_call(request.params)?;
    let events = events.map(|e| Result::<_, Error>::Ok(SseEvent::default().data(e.get())));
    Ok(Sse::new(events))
}
