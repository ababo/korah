mod db;

use crate::db::Db;
use clap::Parser;
use log::error;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("db")]
    Db(#[from] db::Error),
}

#[derive(clap::Parser)]
struct Args {
    #[clap(long, env = "KORAHD_DB")]
    db_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    env_logger::builder().format_timestamp_millis().init();

    let db = if let Some(path) = args.db_path {
        Db::open(path).await
    } else {
        Db::open_in_memory().await
    }?;

    let llm_model: String = db.config_value("llm_model").await?;
    error!("llm model {llm_model}");

    Ok(())
}
