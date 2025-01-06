use crate::{
    tool::{Error, Event, Params, Tool},
    util::fmt::ErrorChainDisplay,
};
use log::info;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::{
    spawn,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
};

/// A tool for finding files on the local file system.
pub struct FindFiles {}

impl FindFiles {
    /// Creates a FindFiles instance.
    pub fn new() -> Self {
        FindFiles {}
    }
}

/// Parameters specific to the FindFiles tool.
#[derive(Deserialize, JsonSchema)]
#[schemars(rename = "find_files", description = "")]
pub struct FindFilesParams {
    _directory: PathBuf,
}

/// An event specific to the FindFiles tool.
#[derive(Debug, Serialize)]
pub struct FindFilesEvent {
    path: PathBuf,
}

impl Tool for FindFiles {
    type Params = FindFilesParams;
    type Event = FindFilesEvent;

    fn call(
        &self,
        _params: Params<Self::Params>,
    ) -> Result<UnboundedReceiver<Event<Self::Event>>, Error> {
        // TODO: Implement this properly.
        let (sender, receiver) = unbounded_channel();
        spawn(async move {
            for i in 0.. {
                if sender.is_closed() {
                    break;
                }
                log::debug!("iter {i}");
                if i % 10 == 0 {
                    if let Err(err) = sender.send(Event {
                        tool_specific: Self::Event {
                            path: format!("/foo/bar-{i}").into(),
                        },
                    }) {
                        info!(
                            "failed to send find_files event: {}",
                            ErrorChainDisplay(&err)
                        );
                        break;
                    };
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            info!("finished file search");
        });
        Ok(receiver)
    }
}
