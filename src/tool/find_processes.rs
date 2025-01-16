use crate::tool::{Error, Tool};
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags};
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    thread::sleep,
};
use sysinfo::{Process, ProcessRefreshKind, ProcessesToUpdate, System};

/// Parameters specific to the FindProcesses tool.
#[derive(Deserialize, JsonSchema)]
pub struct FindProcessesParams {
    detailed_output: Option<bool>,
    #[schemars(description = "Percentage")]
    max_cpu_usage: Option<f32>,
    #[schemars(description = "In bytes")]
    max_memory: Option<u64>,
    #[schemars(description = "In Bytes")]
    max_read_from_disk: Option<u64>,
    #[schemars(description = "In Bytes")]
    max_written_to_disk: Option<u64>,
    #[schemars(description = "Percentage")]
    min_cpu_usage: Option<f32>,
    #[schemars(description = "In bytes")]
    min_memory: Option<u64>,
    #[schemars(description = "In Bytes")]
    min_read_from_disk: Option<u64>,
    #[schemars(description = "In Bytes")]
    min_written_to_disk: Option<u64>,
    name_regex: Option<String>,
    #[schemars(description = "Zero means any.")]
    tcp_port: Option<u16>,
    #[schemars(description = "Zero means any.")]
    udp_port: Option<u16>,
}

/// An output specific to the FindProcesses tool.
#[derive(Debug, JsonSchema, Serialize)]
pub struct FindProcessesOutput {
    #[serde(flatten)]
    details: Option<FindProcessesOutputDetails>,
    name: String,
    pid: u32,
}

impl FindProcessesOutput {
    pub fn details(&self) -> &FindProcessesOutputDetails {
        self.details.as_ref().unwrap()
    }

    pub fn details_mut(&mut self) -> &mut FindProcessesOutputDetails {
        self.details.as_mut().unwrap()
    }
}

#[derive(Debug, JsonSchema, Serialize)]
pub struct FindProcessesOutputDetails {
    cmd: Vec<String>,
    cpu_usage: f32,
    exe: Option<PathBuf>,
    memory: u64,
    read_from_disk: u64,
    tcp_ports: Vec<u16>,
    udp_ports: Vec<u16>,
    written_to_disk: u64,
}

impl From<&Process> for FindProcessesOutput {
    fn from(process: &Process) -> Self {
        let disk_usage = process.disk_usage();
        Self {
            details: Some(FindProcessesOutputDetails {
                cmd: process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect(),
                cpu_usage: process.cpu_usage(),
                exe: process.exe().map(ToOwned::to_owned),
                memory: process.memory(),
                read_from_disk: disk_usage.total_read_bytes,
                tcp_ports: Vec::new(),
                udp_ports: Vec::new(),
                written_to_disk: disk_usage.total_written_bytes,
            }),
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

    fn get_processes() -> HashMap<u32, FindProcessesOutput> {
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

        system
            .processes()
            .iter()
            .map(|(pid, proc)| (pid.as_u32(), proc.into()))
            .collect()
    }

    fn add_net_ports(processes: &mut HashMap<u32, FindProcessesOutput>) -> Result<(), Error> {
        let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
        let sockets_info = get_sockets_info(af_flags, proto_flags)?;

        use netstat2::ProtocolSocketInfo::*;
        for si in sockets_info {
            for pid in si.associated_pids {
                let Some(process) = processes.get_mut(&pid) else {
                    continue;
                };
                match &si.protocol_socket_info {
                    Tcp(tcp_si) => {
                        process.details_mut().tcp_ports.push(tcp_si.local_port);
                    }
                    Udp(udp_si) => {
                        process.details_mut().udp_ports.push(udp_si.local_port);
                    }
                };
            }
        }

        Ok(())
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
        let detailed_output = params.detailed_output.unwrap_or_default();
        let filter: Filter = params.try_into()?;

        let mut processes = Self::get_processes();
        Self::add_net_ports(&mut processes)?;

        let mut processes: Vec<_> = processes
            .into_values()
            .map(FindProcessesOutput::from)
            .filter(|p| filter.is_matching(p))
            .collect();

        if !detailed_output {
            processes.iter_mut().for_each(|p| p.details = None);
        }

        Ok(processes.into_iter())
    }
}

struct Filter {
    max_cpu_usage: Option<f32>,
    max_memory: Option<u64>,
    max_read_from_disk: Option<u64>,
    max_written_to_disk: Option<u64>,
    min_cpu_usage: Option<f32>,
    min_memory: Option<u64>,
    min_read_from_disk: Option<u64>,
    min_written_to_disk: Option<u64>,
    name_regex: Option<Regex>,
    tcp_port: Option<u16>,
    udp_port: Option<u16>,
}

impl Filter {
    fn is_matching(&self, process: &FindProcessesOutput) -> bool {
        if let Some(min_cpu_usage) = self.min_cpu_usage {
            if process.details().cpu_usage < min_cpu_usage {
                return false;
            }
        }

        if let Some(max_cpu_usage) = self.max_cpu_usage {
            if process.details().cpu_usage > max_cpu_usage {
                return false;
            }
        }

        if let Some(min_memory) = self.min_memory {
            if process.details().memory < min_memory {
                return false;
            }
        }

        if let Some(max_memory) = self.max_memory {
            if process.details().memory > max_memory {
                return false;
            }
        }

        if let Some(min_read_from_disk) = self.min_read_from_disk {
            if process.details().read_from_disk < min_read_from_disk {
                return false;
            }
        }

        if let Some(max_read_from_disk) = self.max_read_from_disk {
            if process.details().read_from_disk > max_read_from_disk {
                return false;
            }
        }

        if let Some(min_written_to_disk) = self.min_written_to_disk {
            if process.details().written_to_disk < min_written_to_disk {
                return false;
            }
        }

        if let Some(max_written_to_disk) = self.max_written_to_disk {
            if process.details().written_to_disk > max_written_to_disk {
                return false;
            }
        }

        if let Some(name_regex) = &self.name_regex {
            if !name_regex.is_match(&process.name) {
                return false;
            }
        }

        if let Some(tcp_port) = &self.tcp_port {
            if *tcp_port != 0 {
                if !process.details().tcp_ports.iter().any(|p| p == tcp_port) {
                    return false;
                }
            } else if process.details().tcp_ports.is_empty() {
                return false;
            }
        }

        if let Some(udp_port) = &self.udp_port {
            if *udp_port != 0 {
                if !process.details().udp_ports.iter().any(|p| p == udp_port) {
                    return false;
                }
            } else if process.details().udp_ports.is_empty() {
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
            max_read_from_disk: params.max_read_from_disk,
            max_written_to_disk: params.max_written_to_disk,
            min_cpu_usage: params.min_cpu_usage,
            min_memory: params.min_memory,
            min_read_from_disk: params.min_read_from_disk,
            min_written_to_disk: params.min_written_to_disk,
            name_regex,
            tcp_port: params.tcp_port,
            udp_port: params.udp_port,
        })
    }
}
