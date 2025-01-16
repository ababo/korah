mod find_files;
mod find_processes;

use crate::{
    tool::{find_files::FindFiles, find_processes::FindProcesses},
    util::fmt::ErrorChainDisplay,
};
use log::warn;
use schemars::{schema::RootSchema, schema_for, JsonSchema};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::value::RawValue;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc},
};

/// A tool error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("inconsistent params")]
    InconsistentParams,
    #[error("io error")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
    #[error("netstat2 error")]
    Netstat2(
        #[from]
        #[source]
        netstat2::error::Error,
    ),
    #[error("failed to parse regex")]
    Regex(
        #[from]
        #[source]
        regex::Error,
    ),
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
        shellexpand::path::LookupError<std::env::VarError>,
    ),
}

/// A tool for query processing.
pub trait Tool {
    /// A tool-specific parameters.
    type Params;

    /// A tool-specific output.
    type Output;

    /// A tool name.
    fn name(&self) -> &'static str;

    /// An optional tool description.
    fn description(&self) -> Option<&'static str> {
        None
    }

    /// Calls the tool with given parameters getting an output iterator.
    fn call(
        &self,
        params: Self::Params,
        cancel: Arc<AtomicBool>,
    ) -> Result<impl Iterator<Item = Self::Output> + 'static, Error>;
}

/// A tool metadata.
#[derive(Clone)]
pub struct ToolMeta {
    pub name: String,
    pub description: Option<String>,
    pub params_schema: RootSchema,
    pub _output_schema: RootSchema,
}

/// A tool wrapper for dynamic dispatch.
pub trait DynTool {
    /// Calls the tool with given parameters getting an output iterator.
    fn call(
        &self,
        params: Box<RawValue>,
        cancel: Arc<AtomicBool>,
    ) -> Result<Box<dyn Iterator<Item = Box<RawValue>> + 'static>, Error>;

    /// Tool metadata.
    fn meta(&self) -> ToolMeta;
}

impl<T> DynTool for T
where
    T: Tool,
    T::Params: DeserializeOwned + JsonSchema,
    T::Output: Debug + JsonSchema + Serialize + 'static,
{
    fn call(
        &self,
        params: Box<RawValue>,
        cancel: Arc<AtomicBool>,
    ) -> Result<Box<dyn Iterator<Item = Box<RawValue>>>, Error> {
        let params = serde_json::from_str(params.get())?;
        let iter = Tool::call(self, params, cancel)?;
        Ok(Box::new(iter.filter_map(|o| {
            match serde_json::to_string(&o).and_then(RawValue::from_string) {
                Ok(output) => Some(output),
                Err(err) => {
                    warn!(
                        "failed to serialize tool output {o:?}: {}",
                        ErrorChainDisplay(&err)
                    );
                    None
                }
            }
        })))
    }

    fn meta(&self) -> ToolMeta {
        ToolMeta {
            name: Tool::name(self).to_owned(),
            description: Tool::description(self).map(ToOwned::to_owned),
            params_schema: schema_for!(T::Params),
            _output_schema: schema_for!(T::Output),
        }
    }
}

/// A mapping from tool names to their corresponding tool instances.
pub type DynTools = HashMap<&'static str, Box<dyn DynTool>>;

macro_rules! add_tool {
    ($tools:expr, $tool:expr) => {
        let tool = $tool;
        $tools.insert(tool.name(), Box::new(tool));
    };
}

/// Creates API tools.
pub fn create_tools() -> DynTools {
    let mut tools = DynTools::new();
    add_tool!(tools, FindFiles::new());
    add_tool!(tools, FindProcesses::new());
    tools
}
