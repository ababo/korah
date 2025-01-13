use chrono::{NaiveDateTime, Utc};
use serde::Serialize;
use sys_locale::get_locale;
use whoami::username;

/// An LLM query context.
#[derive(Serialize)]
pub struct Context {
    os_name: &'static str,
    system_locale: String,
    time_now: NaiveDateTime,
    username: String,
}

impl Context {
    /// Creates a new Context instance.
    pub fn new() -> Context {
        Context {
            os_name: std::env::consts::OS,
            system_locale: get_locale().unwrap_or("en-US".to_owned()),
            time_now: Utc::now().naive_utc(),
            username: username(),
        }
    }
}
