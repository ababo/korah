use serde::Serialize;
use std::time::SystemTime;
use sys_locale::get_locale;
use whoami::username;

/// An LLM query context.
#[derive(Serialize)]
pub struct Context {
    system_locale: String,
    time_now: SystemTime,
    username: String,
}

impl Context {
    /// Creates a new Context instance.
    pub fn new() -> Context {
        Context {
            system_locale: get_locale().unwrap_or("en-US".to_owned()),
            time_now: SystemTime::now(),
            username: username(),
        }
    }
}
