mod db;
mod llm;
mod util;

use crate::{
    db::Db,
    llm::{ollama::Ollama, Llm},
    util::fmt::ErrorChainDisplay,
};
use clap::Parser;
use log::{error, info, LevelFilter};
use std::{path::PathBuf, process::exit};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("db")]
    Db(
        #[from]
        #[source]
        crate::db::Error,
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
        .filter_level(LevelFilter::Info)
        .format_timestamp_millis()
        .init();

    let db = if let Some(path) = args.db_path {
        Db::open(path).await
    } else {
        Db::open_in_memory().await
    }?;

    let ollama_url = db.config_value("ollama_url").await?;
    let llm_model: String = db.config_value("llm_model").await?;

    let ollama = Ollama::new(ollama_url)?;

    info!("started preparing llm model '{llm_model}'");
    ollama.prepare_model(&llm_model).await?;
    info!("finished preparing llm model '{llm_model}'");

    Ok(())
}
