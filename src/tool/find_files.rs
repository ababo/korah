use crate::{
    tool::{Error, Tool},
    util::fmt::ErrorChainDisplay,
};
use chrono::{DateTime, Utc};
use log::warn;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::{read_dir, Metadata, ReadDir},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};

/// Parameters specific to the FindFiles tool.
#[derive(Deserialize, JsonSchema)]
pub struct FindFilesParams {
    in_directory: PathBuf,
    is_directory: Option<bool>,
    is_symlink: Option<bool>,
    #[schemars(description = "In bytes")]
    min_size: Option<u64>,
    #[schemars(description = "In ISO 8601 format")]
    min_time_created: Option<DateTime<Utc>>,
    #[schemars(description = "In bytes")]
    max_size: Option<u64>,
    #[schemars(description = "In ISO 8601 format")]
    max_time_created: Option<DateTime<Utc>>,
    #[schemars(description = "In ISO 8601 format")]
    min_time_modified: Option<DateTime<Utc>>,
    #[schemars(description = "In ISO 8601 format")]
    max_time_modified: Option<DateTime<Utc>>,
    #[schemars(description = "RE2-compatible.")]
    name_regex: Option<String>,
}

/// An output specific to the FindFiles tool.
#[derive(Debug, JsonSchema, Serialize)]
pub struct FindFilesOutput {
    path: PathBuf,
}

/// A tool for finding files on the local file system.
pub struct FindFiles;

impl FindFiles {
    /// Creates a FindFiles instance.
    pub fn new() -> Self {
        FindFiles
    }
}

impl Tool for FindFiles {
    type Params = FindFilesParams;
    type Output = FindFilesOutput;

    fn name(&self) -> &'static str {
        "find_files"
    }

    fn call(
        &self,
        params: FindFilesParams,
        cancel: Arc<AtomicBool>,
    ) -> Result<impl Iterator<Item = FindFilesOutput> + 'static, Error> {
        let in_directory = shellexpand::path::full(&params.in_directory)?;
        let entries = read_dir(&in_directory)?;
        let filter = params.try_into()?;
        Ok(FindFilesIterator {
            filter,
            cancel,
            entries_stack: vec![entries],
        })
    }
}

struct Filter {
    is_directory: Option<bool>,
    is_symlink: Option<bool>,
    min_size: Option<u64>,
    min_time_created: Option<SystemTime>,
    max_size: Option<u64>,
    max_time_created: Option<SystemTime>,
    min_time_modified: Option<SystemTime>,
    max_time_modified: Option<SystemTime>,
    name_regex: Option<Regex>,
}

impl Filter {
    fn is_matching(&self, path: &str, name: &OsStr, mut meta: Metadata) -> bool {
        if let Some(is_symlink) = self.is_symlink {
            if meta.is_symlink() != is_symlink {
                return false;
            }
        }

        if self.min_time_created.is_some() || self.max_time_created.is_some() {
            let time_created = match meta.created() {
                Ok(time) => time,
                Err(err) => {
                    warn!(
                        "failed to get created time for {path}: {}",
                        ErrorChainDisplay(&err)
                    );
                    return false;
                }
            };
            if let Some(min_time_created) = self.min_time_created {
                if time_created < min_time_created {
                    return false;
                }
            }
            if let Some(max_time_created) = self.max_time_created {
                if time_created > max_time_created {
                    return false;
                }
            }
        }

        if self.min_time_modified.is_some() || self.max_time_modified.is_some() {
            let time_modified = match meta.modified() {
                Ok(time) => time,
                Err(err) => {
                    warn!(
                        "failed to get modified time for {path}: {}",
                        ErrorChainDisplay(&err)
                    );
                    return false;
                }
            };
            if let Some(min_time_modified) = self.min_time_modified {
                if time_modified < min_time_modified {
                    return false;
                }
            }
            if let Some(max_time_modified) = self.max_time_modified {
                if time_modified > max_time_modified {
                    return false;
                }
            }
        }

        if let Some(name_regex) = &self.name_regex {
            if let Some(name) = name.to_str() {
                if !name_regex.is_match(name) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Here we resolve a possible symlink.
        // The following checks are only related to the final target.
        if meta.is_symlink() {
            match std::fs::metadata(path) {
                Ok(m) => meta = m,
                Err(err) => {
                    warn!(
                        "failed to get meta for symlink {path}: {}",
                        ErrorChainDisplay(&err)
                    );
                }
            }
        }

        if let Some(is_directory) = self.is_directory {
            if meta.is_dir() != is_directory {
                return false;
            }
        }

        if let Some(min_size) = self.min_size {
            if meta.len() < min_size {
                return false;
            }
        }

        if let Some(max_size) = self.max_size {
            if meta.len() > max_size {
                return false;
            }
        }

        true
    }
}

impl TryFrom<FindFilesParams> for Filter {
    type Error = Error;

    fn try_from(params: FindFilesParams) -> Result<Self, Error> {
        let min_time_created = params.min_time_created.map(Into::into);
        let max_time_created = params.max_time_created.map(Into::into);
        let min_time_modified = params.min_time_modified.map(Into::into);
        let max_time_modified = params.max_time_modified.map(Into::into);
        let name_regex = params.name_regex.as_deref().map(Regex::new).transpose()?;
        Ok(Self {
            is_directory: params.is_directory,
            is_symlink: params.is_symlink,
            min_size: params.min_size,
            min_time_created,
            max_size: params.max_size,
            max_time_created,
            min_time_modified,
            max_time_modified,
            name_regex,
        })
    }
}

pub struct FindFilesIterator {
    filter: Filter,
    cancel: Arc<AtomicBool>,
    entries_stack: Vec<ReadDir>,
}

impl Iterator for FindFilesIterator {
    type Item = FindFilesOutput;

    fn next(&mut self) -> Option<FindFilesOutput> {
        loop {
            if self.cancel.load(Ordering::SeqCst) {
                return None;
            }

            let entries = self.entries_stack.last_mut()?;

            let Some(entry_result) = entries.next() else {
                self.entries_stack.pop();
                continue;
            };

            let entry = match entry_result {
                Ok(entry) => entry,
                Err(err) => {
                    warn!("failed to read dir entry: {}", ErrorChainDisplay(&err));
                    continue;
                }
            };

            let path = entry.path().to_str().unwrap_or("?").to_owned();

            let meta = match entry.metadata() {
                Ok(meta) => meta,
                Err(err) => {
                    warn!(
                        "failed to read meta for {path}: {}",
                        ErrorChainDisplay(&err)
                    );
                    continue;
                }
            };

            if meta.is_dir() {
                match read_dir(entry.path()) {
                    Ok(entries) => {
                        self.entries_stack.push(entries);
                    }
                    Err(err) => {
                        warn!("failed to read dir {path}: {}", ErrorChainDisplay(&err));
                    }
                };
            }

            if self.filter.is_matching(&path, &entry.file_name(), meta) {
                return Some(FindFilesOutput { path: entry.path() });
            }
        }
    }
}
