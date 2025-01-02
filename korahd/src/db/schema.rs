use crate::db::{Db, Error};

impl Db {
    pub async fn schema_version(&self) -> Result<Option<u32>, Error> {
        self.conn
            .call(|conn| {
                let missing: bool = conn.query_row(
                    "SELECT COUNT(*)
                       FROM sqlite_master
                      WHERE type='table' AND name='schema'",
                    [],
                    |row| {
                        let count: i32 = row.get(0)?;
                        Ok(count == 0)
                    },
                )?;
                if missing {
                    return Ok(None);
                }

                let version = conn.query_row(
                    "SELECT version
                       FROM schema",
                    [],
                    |row| row.get(0),
                )?;

                Ok(Some(version))
            })
            .await
            .map_err(Into::into)
    }
}
