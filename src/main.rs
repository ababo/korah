mod config;
mod context;
mod llm;
mod tool;
mod util;

use crate::{
    context::Context, llm::ollama::Ollama, tool::create_tools, util::fmt::ErrorChainDisplay,
};
use clap::{
    builder::{IntoResettable, OsStr},
    Parser,
};
use config::{Config, LlmApi};
use log::{error, info, warn};
use std::{
    collections::HashMap,
    ffi::OsString,
    path::PathBuf,
    process::exit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use strfmt::strfmt;

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
    #[error("{0} config missing")]
    LlmConfigMissing(&'static str),
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
    #[clap(long, short = 'a', help = "LLM API [default in config].")]
    llm_api: Option<LlmApi>,
    #[clap(help = "Query in human language")]
    query: String,
    #[clap(
        long,
        short = 'n',
        help = "Number of tries to derive a tool call",
        default_value = "3"
    )]
    num_derive_tries: u32,
}

fn default_config_path() -> impl IntoResettable<OsStr> {
    #[cfg(unix)]
    let paths: Vec<OsString> = vec![".".into(), "~/.config".into(), "/etc".into()];

    #[cfg(windows)]
    let paths: Vec<OsString> = vec![
        ".".into(),
        env::var_os("USERPROFILE").unwrap(),
        env::var_os("SystemDrive").unwrap(),
    ];

    const BASENAME: &str = "korah.toml";
    for path in paths {
        let filename = PathBuf::from(path).join(BASENAME);
        if filename.exists() {
            return filename.into_os_string().into_resettable();
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
    let llm = match config.llm.api {
        LlmApi::Ollama => {
            let Some(ollama_config) = config.llm.ollama else {
                return Err(Error::LlmConfigMissing("ollama"));
            };
            Ollama::new_boxed(ollama_config, tools_meta)
        }
    };

    let context = serde_json::to_string(&Context::new()).unwrap();
    let mut vars = HashMap::new();
    vars.insert("context".to_owned(), context);
    vars.insert("query".to_owned(), args.query.clone());
    let query = strfmt(&config.llm.query_fmt, &vars).unwrap();

    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_cloned = cancel.clone();
    ctrlc::set_handler(move || {
        warn!("received cancelling request");
        cancel_cloned.store(true, Ordering::SeqCst);
    })
    .unwrap();

    let outputs = 'a: {
        for _ in 0..args.num_derive_tries {
            let Some(call) = llm.derive_tool_call(&query)? else {
                continue;
            };
            info!("derived call {}({})", call.name, call.params);

            let Some(tool) = tools.get(&call.name.as_str()) else {
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
