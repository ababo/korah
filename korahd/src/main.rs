mod api;
mod db;
mod llm;
mod tool;
mod util;

use crate::{api::create_api, db::Db, llm::ollama::Ollama, util::fmt::ErrorChainDisplay};
use clap::Parser;
use log::{error, info, LevelFilter};
use std::{net::SocketAddr, path::PathBuf, process::exit};
use tokio::net::TcpListener;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("db")]
    Db(
        #[from]
        #[source]
        crate::db::Error,
    ),
    #[error("io")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
    #[error("llm")]
    Llm(
        #[from]
        #[source]
        crate::llm::Error,
    ),
}

#[derive(clap::Parser)]
struct Args {
    #[clap(long, env = "KORAHD_DB")]
    db_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Err(err) = run(args).await {
        error!("failed to run: {}", ErrorChainDisplay(&err));
        exit(1);
    }
}

async fn run(args: Args) -> Result<(), Error> {
    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    let db = if let Some(path) = args.db_path {
        Db::open(path).await
    } else {
        Db::open_in_memory().await
    }?;

    let ollama_url = db.config_value("ollama_url").await?;
    let llm_model: String = db.config_value("llm_model").await?;

    let llm = Ollama::new_boxed(ollama_url)?;

    info!("started preparing llm model '{llm_model}'");
    llm.prepare_model(&llm_model).await?;
    info!("finished preparing llm model '{llm_model}'");

    let api_address: SocketAddr = db.config_value("api_address").await?;
    let listener = TcpListener::bind(api_address).await?;
    let api = create_api(db, llm);

    axum::serve(listener, api).await?;

    Ok(())
}
