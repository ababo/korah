use crate::tool::{Error, Tool};
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    thread::sleep,
};
use sysinfo::{Process, ProcessRefreshKind, ProcessesToUpdate, System};

/// Parameters specific to the FindProcesses tool.
#[derive(Deserialize, JsonSchema)]
pub struct FindProcessesParams {
    #[schemars(description = "Percentage")]
    max_cpu_usage: Option<f32>,
    #[schemars(description = "In bytes")]
    max_memory: Option<u64>,
    #[schemars(description = "Percentage")]
    min_cpu_usage: Option<f32>,
    #[schemars(description = "In bytes")]
    min_memory: Option<u64>,
    name_regex: Option<String>,
}

/// An output specific to the FindProcesses tool.
#[derive(Debug, JsonSchema, Serialize)]
pub struct FindProcessesOutput {
    cmd: Vec<String>,
    cpu_usage: f32,
    exe: Option<PathBuf>,
    memory: u64,
    name: String,
    pid: u32,
}

impl From<&Process> for FindProcessesOutput {
    fn from(process: &Process) -> Self {
        Self {
            cmd: process
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect(),
            cpu_usage: process.cpu_usage(),
            exe: process.exe().map(ToOwned::to_owned),
            memory: process.memory(),
            name: process.name().to_string_lossy().to_string(),
            pid: process.pid().as_u32(),
        }
    }
}

/// A tool for finding processes running in the system.
pub struct FindProcesses;

impl FindProcesses {
    /// Creates a FindProcesses instance.
    pub fn new() -> Self {
        FindProcesses
    }
}

impl Tool for FindProcesses {
    type Params = FindProcessesParams;
    type Output = FindProcessesOutput;

    fn name(&self) -> &'static str {
        "find_processes"
    }

    fn call(
        &self,
        params: FindProcessesParams,
        _cancel: Arc<AtomicBool>,
    ) -> Result<impl Iterator<Item = FindProcessesOutput> + 'static, Error> {
        let filter: Filter = params.try_into()?;

        let mut system = System::new_all();

        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu(),
        );

        sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu(),
        );

        let processes: Vec<_> = system
            .processes()
            .values()
            .map(FindProcessesOutput::from)
            .filter(|p| filter.is_matching(p))
            .collect();

        Ok(processes.into_iter())
    }
}

struct Filter {
    max_cpu_usage: Option<f32>,
    max_memory: Option<u64>,
    min_cpu_usage: Option<f32>,
    min_memory: Option<u64>,
    name_regex: Option<Regex>,
}

impl Filter {
    fn is_matching(&self, process: &FindProcessesOutput) -> bool {
        if let Some(min_cpu_usage) = self.min_cpu_usage {
            if process.cpu_usage < min_cpu_usage {
                return false;
            }
        }

        if let Some(max_cpu_usage) = self.max_cpu_usage {
            if process.cpu_usage > max_cpu_usage {
                return false;
            }
        }

        if let Some(min_memory) = self.min_memory {
            if process.memory < min_memory {
                return false;
            }
        }

        if let Some(max_memory) = self.max_memory {
            if process.memory > max_memory {
                return false;
            }
        }

        if let Some(name_regex) = &self.name_regex {
            if !name_regex.is_match(&process.name) {
                return false;
            }
        }

        true
    }
}

impl TryFrom<FindProcessesParams> for Filter {
    type Error = Error;

    fn try_from(params: FindProcessesParams) -> Result<Self, Error> {
        let name_regex = params.name_regex.as_deref().map(Regex::new).transpose()?;
        Ok(Self {
            max_cpu_usage: params.max_cpu_usage,
            max_memory: params.max_memory,
            min_cpu_usage: params.min_cpu_usage,
            min_memory: params.min_memory,
            name_regex,
        })
    }
}
