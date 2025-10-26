mod config;
mod error;

use crate::error::MachineContextError;
use capsula_core::captured::Captured;
use capsula_core::error::CoreResult;
use capsula_core::hook::{Context, ContextFactory, RuntimeParams};
use config::MachineContextFactory;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

pub const KEY: &str = "capture-machine";

#[derive(Debug, Default)]
pub struct MachineContext;

#[derive(Debug)]
pub struct CpuInfo {
    name: String,
    vender_id: String,
    brand: String,
    frequency_mhz: u64,
}

#[derive(Debug)]
pub struct MachineCaptured {
    pub os: String,
    pub os_version: String,
    pub kernel_version: String,
    pub architecture: String,
    pub cpus: Vec<CpuInfo>,
    // pub cpu_cores: usize,
    pub total_memory: usize,
    // pub user: String,
    pub hostname: String,
}

impl Context for MachineContext {
    type Output = MachineCaptured;

    fn run(&self, _params: &RuntimeParams) -> CoreResult<Self::Output> {
        let os = System::name().ok_or(MachineContextError::OsInfoError)?;
        let os_version = System::os_version().ok_or(MachineContextError::OsInfoError)?;
        let kernel_version = System::kernel_version().ok_or(MachineContextError::OsInfoError)?;
        let architecture = std::env::consts::ARCH.to_string();

        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_frequency())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        let cpus = system
            .cpus()
            .iter()
            .map(|cpu| CpuInfo {
                name: cpu.name().to_string(),
                vender_id: cpu.vendor_id().to_string(),
                brand: cpu.brand().to_string(),
                frequency_mhz: cpu.frequency(),
            })
            .collect::<Vec<_>>();

        let total_memory = system.total_memory();
        let hostname = System::host_name().ok_or(MachineContextError::HostnameError)?;

        Ok(MachineCaptured {
            os,
            os_version,
            kernel_version,
            architecture,
            cpus,
            total_memory: total_memory as usize,
            hostname,
        })
    }
}

impl Captured for MachineCaptured {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": KEY,
            "os": self.os,
            "os_version": self.os_version,
            "kernel_version": self.kernel_version,
            "architecture": self.architecture,
            "cpus": self.cpus.iter().map(|cpu| {
                serde_json::json!({
                    "name": cpu.name,
                    "vender_id": cpu.vender_id,
                    "brand": cpu.brand,
                    "frequency_mhz": cpu.frequency_mhz,
                })
            }).collect::<Vec<_>>(),
            "total_memory": self.total_memory,
            "hostname": self.hostname,
        })
    }
}

pub fn create_factory() -> Box<dyn ContextFactory> {
    Box::new(MachineContextFactory)
}
