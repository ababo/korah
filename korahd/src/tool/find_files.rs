use crate::tool::ToolOutput;
use crate::tool::{Error, Tool};
use serde_json::value::RawValue;
use tokio::sync::{mpsc::Sender, oneshot::Receiver};

pub struct FindFiles {}

impl FindFiles {
    pub fn new() -> Self {
        FindFiles {}
    }
}

impl Tool for FindFiles {
    fn call(
        &self,
        _params: &RawValue,
        _cancel: Receiver<()>,
        _output: Sender<ToolOutput>,
    ) -> Result<(), Error> {
        todo!()
    }

    fn params_schema(&self) -> Box<RawValue> {
        todo!()
    }
}
