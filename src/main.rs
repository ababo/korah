mod config;
mod llm;
mod tool;
mod util;

use crate::{
    config::Config,
    llm::{create_llm_client, Context, ToolCall},
    tool::{create_tools, BoxOutputIter},
    tool::{DynTools, ToolMeta},
    util::fmt::ErrorChainDisplay,
};
use clap::{
    builder::{IntoResettable, OsStr},
    Parser,
};
use either::Either;
use log::{debug, error, info, log_enabled, warn};
use std::{
    path::PathBuf,
    process::exit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("processing cancelled")]
    Cancelled,
    #[error("failed to read config")]
    Config(
        #[from]
        #[source]
        crate::config::Error,
    ),
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
    #[error("unknown tool '{0}'")]
    UnknownTool(String),
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
    Config::find_common_path()
        .unwrap_or(Config::COMMON_FILE_BASENAME.into())
        .into_os_string()
}

macro_rules! check_cancel {
    ($cancel: expr) => {
        if $cancel.load(Ordering::SeqCst) {
            return Ok(Either::Left(Box::new(std::iter::empty())));
        }
    };
}

fn derive_and_call_tool(
    config: &Config,
    args: &Args,
    tools: DynTools,
    cancel: Arc<AtomicBool>,
) -> Result<Either<BoxOutputIter, ToolCall>, Error> {
    let contextualized_query = Context::new().contextualize(&config.llm, args.query.clone());
    debug!("contextualized query '{contextualized_query}'");

    let tools_meta: Vec<_> = tools.values().map(|t| t.meta()).collect();
    let llm = create_llm_client(&config.llm)?;

    let outputs = 'a: {
        for _ in 0..config.num_derive_tries {
            check_cancel!(cancel);

            let call = if config.double_pass_derive {
                let tools_stripped_meta: Vec<_> = tools_meta
                    .iter()
                    .cloned()
                    .map(ToolMeta::strip_params)
                    .collect();
                let Some(call) = llm.derive_tool_call(tools_stripped_meta, args.query.clone())?
                else {
                    warn!("no tool name derived");
                    continue;
                };

                let mut tools_meta = tools_meta.clone();
                tools_meta.retain(|t| t.name == call.tool);
                if tools_meta.is_empty() {
                    warn!("unknown derived tool '{}'", call.tool);
                    continue;
                }

                check_cancel!(cancel);

                match llm.derive_tool_call(tools_meta.clone(), contextualized_query.clone())? {
                    Some(call) => call,
                    None => {
                        warn!("no tool call params derived");
                        continue;
                    }
                }
            } else {
                match llm.derive_tool_call(tools_meta.clone(), contextualized_query.clone())? {
                    Some(call) => call,
                    None => {
                        warn!("no tool calls derived");
                        continue;
                    }
                }
            };

            if args.derive_only {
                return Ok(Either::Right(call));
            }

            if log_enabled!(log::Level::Info) {
                let json = serde_json::to_string(&call).unwrap();
                info!("derived call {json}");
            }

            let Some(tool) = tools.get(&call.tool.as_str()) else {
                warn!("unknown derived tool '{}'", call.tool);
                continue;
            };

            match tool.call(call.params, cancel.clone()) {
                Ok(it) => break 'a it,
                Err(err) => warn!("derived call failed: {}", ErrorChainDisplay(&err)),
            }
        }
        return Err(Error::DeriveToolCall);
    };

    Ok(Either::Left(outputs))
}

fn run(args: Args) -> Result<(), Error> {
    env_logger::builder()
        .format_timestamp_millis()
        .parse_default_env()
        .init();

    let config = Config::read(&args.config_path)?;
    let tools = create_tools();

    let cancel = Arc::new(AtomicBool::new(false));
    {
        let cancel_cloned = cancel.clone();
        ctrlc::set_handler(move || {
            warn!("received cancelling request");
            cancel_cloned.store(true, Ordering::SeqCst);
        })
        .unwrap();
    }

    let outputs = if let Ok(call) = serde_json::from_str::<ToolCall>(&args.query) {
        info!("interpreted query as a tool call");
        let Some(tool) = tools.get(&call.tool.as_str()) else {
            return Err(Error::UnknownTool(call.tool));
        };
        tool.call(call.params, cancel.clone())?
    } else {
        match derive_and_call_tool(&config, &args, tools, cancel.clone())? {
            Either::Left(outputs) => outputs,
            Either::Right(call) => {
                // The derive_only case.
                let json = serde_json::to_string(&call).unwrap();
                println!("{json}");
                return Ok(());
            }
        }
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
