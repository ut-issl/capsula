mod error;

use crate::error::MachineHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
pub struct MachineHookConfig {}

#[derive(Debug, Default)]
pub struct MachineHook {
    config: MachineHookConfig,
}

#[derive(Debug, Serialize)]
pub struct CpuInfo {
    name: String,
    vender_id: String,
    brand: String,
    frequency_mhz: u64,
}

#[derive(Debug, Serialize)]
pub struct MachineCaptured {
    os: String,
    os_version: String,
    kernel_version: String,
    architecture: String,
    cpus: Vec<CpuInfo>,
    // pub cpu_cores: usize,
    total_memory: u64,
    // pub user: String,
    hostname: String,
}

impl<P> Hook<P> for MachineHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-machine";

    type Config = MachineHookConfig;
    type Output = MachineCaptured;

    fn from_config(
        _config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        Ok(Self {
            config: MachineHookConfig {},
        })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        _metadata: &PreparedRun,
        _params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        debug!("MachineHook: Gathering system information");
        let os = System::name().ok_or(MachineHookError::OsInfoError)?;
        let os_version = System::os_version().ok_or(MachineHookError::OsInfoError)?;
        let kernel_version = System::kernel_version().ok_or(MachineHookError::OsInfoError)?;
        let architecture = std::env::consts::ARCH.to_string();
        debug!(
            "MachineHook: OS: {} {}, Kernel: {}, Arch: {}",
            os, os_version, kernel_version, architecture
        );

        debug!("MachineHook: Querying CPU and memory information");
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
        let hostname = System::host_name().ok_or(MachineHookError::HostnameError)?;
        debug!(
            "MachineHook: Found {} CPUs, {} bytes total memory, hostname: {}",
            cpus.len(),
            total_memory,
            hostname
        );

        Ok(MachineCaptured {
            os,
            os_version,
            kernel_version,
            architecture,
            cpus,
            total_memory,
            hostname,
        })
    }
}

impl Captured for MachineCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}
