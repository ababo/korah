use crate::db::{Db, Error};
use std::{error::Error as StdError, str::FromStr};

impl Db {
    pub async fn config_value<'a, T, E>(&self, key: &'static str) -> Result<T, Error>
    where
        T: FromStr<Err = E>,
        E: StdError + Send + 'static,
    {
        let value = self
            .conn
            .call(move |conn| {
                let value: String = conn.query_row(
                    "SELECT value
                       FROM config
                      WHERE key = ?",
                    [key],
                    |row| row.get(0),
                )?;
                Ok(value)
            })
            .await?;

        T::from_str(&value).map_err(|e| Error::ConfigValueParse(Box::new(e)))
    }
}
