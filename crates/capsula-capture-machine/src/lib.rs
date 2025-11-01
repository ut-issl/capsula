mod config;
mod error;

use crate::error::MachineHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use config::MachineHookConfig;
use serde::Serialize;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

pub const KEY: &str = "capture-machine";

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

impl<P> Hook<P> for MachineHook
where
    P: PhaseMarker,
{
    const KEY: &'static str = KEY;

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

    fn id(&self) -> String {
        KEY.to_string()
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        _metadata: &PreparedRun,
        _params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        let os = System::name().ok_or(MachineHookError::OsInfoError)?;
        let os_version = System::os_version().ok_or(MachineHookError::OsInfoError)?;
        let kernel_version = System::kernel_version().ok_or(MachineHookError::OsInfoError)?;
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
        let hostname = System::host_name().ok_or(MachineHookError::HostnameError)?;

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
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}
