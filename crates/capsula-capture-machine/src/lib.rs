mod error;

use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookOutcome, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tracing::{debug, warn};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kernel_version: Option<String>,
    architecture: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpus: Option<Vec<CpuInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_memory: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
}

/// Turns the result of a `sysinfo` query into an optional field.
///
/// Information that cannot be retrieved on this machine (e.g. unsupported on the platform)
/// is omitted from the output instead of failing the hook.
fn optional<T>(field: &str, result: Result<T, sysinfo::Error>) -> Option<T> {
    result
        .inspect_err(|error| warn!("MachineHook: Omitting {field}: {error}"))
        .ok()
}

impl<P> Hook<P> for MachineHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-machine";

    type Config = MachineHookConfig;
    type Output = MachineCaptured;

    fn from_config(
        config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: MachineHookConfig = serde_json::from_value(config.clone())?;
        Ok(Self { config })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        _metadata: &PreparedRun,
        _params: &RuntimeParams<P>,
    ) -> CapsulaResult<HookOutcome<Self::Output>> {
        debug!("MachineHook: Gathering system information");
        let os = optional("os", System::name());
        let os_version = optional("os_version", System::os_version());
        let kernel_version = optional("kernel_version", System::kernel_version());
        let architecture = std::env::consts::ARCH.to_string();
        debug!(
            "MachineHook: OS: {:?} {:?}, Kernel: {:?}, Arch: {}",
            os, os_version, kernel_version, architecture
        );

        debug!("MachineHook: Querying CPU and memory information");
        let system = optional(
            "cpus and total_memory",
            System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::nothing().with_frequency())
                    .with_memory(MemoryRefreshKind::nothing().with_ram()),
            ),
        );
        let cpus = system.as_ref().map(|system| {
            system
                .cpus()
                .iter()
                .map(|cpu| CpuInfo {
                    name: cpu.name().to_string(),
                    vender_id: cpu.vendor_id().to_string(),
                    brand: cpu.brand().to_string(),
                    frequency_mhz: cpu.frequency(),
                })
                .collect::<Vec<_>>()
        });
        let total_memory = system.as_ref().map(System::total_memory);
        let hostname = optional("hostname", System::host_name());
        debug!(
            "MachineHook: Found {:?} CPUs, {:?} bytes total memory, hostname: {:?}",
            cpus.as_ref().map(Vec::len),
            total_memory,
            hostname
        );

        Ok(HookOutcome::success(MachineCaptured {
            os,
            os_version,
            kernel_version,
            architecture,
            cpus,
            total_memory,
            hostname,
        }))
    }
}

impl Captured for MachineCaptured {
    fn serialize_json(
        &self,
    ) -> Result<serde_json::Map<String, serde_json::Value>, serde_json::Error> {
        capsula_core::captured::to_object(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_keeps_retrieved_value() {
        assert_eq!(optional("os", Ok::<_, sysinfo::Error>(1)), Some(1));
    }

    #[test]
    fn optional_omits_unavailable_value() {
        assert_eq!(
            optional::<u64>("os", Err(sysinfo::Error::Unsupported)),
            None
        );
    }

    #[test]
    fn unavailable_fields_are_omitted_from_output() {
        let captured = MachineCaptured {
            os: None,
            os_version: None,
            kernel_version: None,
            architecture: "aarch64".to_string(),
            cpus: None,
            total_memory: None,
            hostname: None,
        };

        let json = captured.serialize_json().expect("serialization succeeds");

        assert_eq!(
            json.keys().collect::<Vec<_>>(),
            vec!["architecture"],
            "only the always-available field should remain"
        );
    }
}
