mod find_files;

use find_files::FindFiles;
use serde_json::value::RawValue;
use std::collections::HashMap;
use tokio::sync::{mpsc::Sender, oneshot::Receiver};

#[derive(thiserror::Error, Debug)]
pub enum Error {}

#[derive(serde::Serialize, Debug)]
pub struct ToolOutput {
    #[serde(flatten)]
    tool_output: Box<RawValue>,
}

pub trait Tool {
    fn call(
        &self,
        params: &RawValue,
        cancel: Receiver<()>,
        output: Sender<ToolOutput>,
    ) -> Result<(), Error>;

    fn params_schema(&self) -> Box<RawValue>;
}

pub fn create_tools() -> HashMap<String, Box<dyn Tool>> {
    fn tool(name: &str, tool: impl Tool + 'static) -> (String, Box<dyn Tool>) {
        (name.to_string(), Box::new(tool))
    }
    let tools = [tool("find_files", FindFiles::new())];
    tools.into_iter().collect()
}
