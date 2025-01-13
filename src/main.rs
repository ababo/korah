mod llm;
mod tool;
mod util;

use crate::{
    llm::{create_llm_client, Context, LlmConfig},
    tool::create_tools,
    util::fmt::ErrorChainDisplay,
};
use clap::{
    builder::{IntoResettable, OsStr},
    Parser,
};
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::{
    path::PathBuf,
    process::exit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// A program configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub num_derive_tries: u32,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("processing cancelled")]
    Cancelled,
    #[error("failed to derive tool call")]
    DeriveToolCall,
    #[error("llm error")]
    Llm(
        #[from]
        #[source]
        crate::llm::Error,
    ),
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
    #[error("tool error")]
    Tool(
        #[from]
        #[source]
        crate::tool::Error,
    ),
}

#[derive(clap::Parser)]
struct Args {
    #[clap(long, short='c', help="Path to config", default_value=default_config_path())]
    config_path: PathBuf,
    #[clap(
        long,
        short = 'd',
        help = "Derive tool call only",
        default_value = "false"
    )]
    derive_only: bool,
    #[clap(help = "Query in human language")]
    query: String,
}

fn default_config_path() -> impl IntoResettable<OsStr> {
    #[cfg(unix)]
    let paths = vec![".", "$HOME/.config", "/etc"];

    #[cfg(windows)]
    let paths = vec![".", "$USERPROFILE", "$SystemDrive"];

    const BASENAME: &str = "korah.toml";
    for path in paths {
        let filename = PathBuf::from(path).join(BASENAME);
        let filename = shellexpand::path::env(&filename).unwrap();
        if filename.exists() {
            return filename.to_path_buf().into_os_string().into_resettable();
        }
    }
    BASENAME.into_resettable()
}

fn run(args: Args) -> Result<(), Error> {
    env_logger::builder()
        .format_timestamp_millis()
        .parse_default_env()
        .init();

    let config: Config = {
        let s = std::fs::read_to_string(args.config_path)?;
        toml::from_str(&s)?
    };

    let tools = create_tools();
    let tools_meta: Vec<_> = tools.values().map(|t| t.meta()).collect();
    let llm = create_llm_client(&config.llm, tools_meta)?;
    let query = Context::new().contextualize(&config.llm, args.query);
    debug!("contextualized query '{query}'");

    let cancel = Arc::new(AtomicBool::new(false));
    {
        let cancel_cloned = cancel.clone();
        ctrlc::set_handler(move || {
            warn!("received cancelling request");
            cancel_cloned.store(true, Ordering::SeqCst);
        })
        .unwrap();
    }

    let outputs = 'a: {
        for _ in 0..config.num_derive_tries {
            let Some(call) = llm.derive_tool_call(&query)? else {
                continue;
            };

            let call_json = serde_json::to_string(&call).unwrap();
            if args.derive_only {
                println!("{call_json}");
                return Ok(());
            }
            info!("derived call {call_json}");

            let Some(tool) = tools.get(&call.tool.as_str()) else {
                warn!("derived tool not found");
                continue;
            };

            match tool.call(call.params, cancel.clone()) {
                Ok(it) => break 'a it,
                Err(err) => warn!("derived call failed: {}", ErrorChainDisplay(&err)),
            }
        }
        return Err(Error::DeriveToolCall);
    };

    for output in outputs {
        println!("{}", output.get());
    }

    if cancel.load(Ordering::SeqCst) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        error!("{}", ErrorChainDisplay(&err));
        exit(1);
    }
}
