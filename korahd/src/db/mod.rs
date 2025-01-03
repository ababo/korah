mod config;
mod schema;

use std::{error::Error as StdError, path::Path};
use tokio_rusqlite::Connection;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("config value parse")]
    ConfigValueParse(#[source] Box<dyn StdError + Send>),
    #[error("tokio_rusqlite")]
    TokioRusqlite(
        #[from]
        #[source]
        tokio_rusqlite::Error,
    ),
    #[error("unsupported schema version")]
    UnsupportedSchemaVersion,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let conn = Connection::open(path).await?;

        let db = Db { conn };
        if let Some(version) = db.schema_version().await? {
            if version != 0 {
                return Err(Error::UnsupportedSchemaVersion);
            }
        } else {
            db.conn
                .call(|conn| {
                    let sql: &str = include_str!("schema.sql");
                    conn.execute_batch(sql).map_err(Into::into)
                })
                .await?;
        }

        Ok(db)
    }

    pub async fn open_in_memory() -> Result<Self, Error> {
        Self::open(":memory:").await
    }
}
