use crate::agent_modules::utils::collect_public_ip_addresses;
use nodenexus_common::agent_service::{AgentHandshake, OsType};
use crate::version::VERSION;
use sysinfo::System;
use uuid::Uuid;

pub async fn create_handshake_payload() -> AgentHandshake {
    let os_type_proto = if cfg!(target_os = "linux") {
        OsType::Linux
    } else if cfg!(target_os = "macos") {
        OsType::Macos
    } else if cfg!(target_os = "windows") {
        OsType::Windows
    } else {
        OsType::default()
    };

    let (public_ips, country_opt) = collect_public_ip_addresses().await;

    let mut sys = System::new();
    sys.refresh_cpu_list(sysinfo::CpuRefreshKind::everything());
    sys.refresh_memory_specifics(sysinfo::MemoryRefreshKind::everything());

    let cpu_static_info_opt = sys
        .cpus()
        .first()
        .map(|cpu| nodenexus_common::agent_service::CpuStaticInfo {
            name: cpu.name().to_string(),
            frequency: cpu.frequency(),
            vendor_id: cpu.vendor_id().to_string(),
            brand: cpu.brand().to_string(),
        });

    AgentHandshake {
        agent_id_hint: Uuid::new_v4().to_string(),
        agent_version: VERSION.to_string(),
        os_type: i32::from(os_type_proto),
        os_name: System::name().unwrap_or_else(|| "N/A".to_string()),
        arch: System::cpu_arch(),
        hostname: System::host_name().unwrap_or_else(|| "N/A".to_string()),
        public_ip_addresses: public_ips,
        kernel_version: System::kernel_version().unwrap_or_else(|| "N/A".to_string()),
        os_version_detail: System::os_version().unwrap_or_else(|| "N/A".to_string()),
        long_os_version: System::long_os_version().unwrap_or_else(|| "N/A".to_string()),
        distribution_id: System::distribution_id(),
        physical_core_count: System::physical_core_count().map(|c| c as u32),
        total_memory_bytes: Some(sys.total_memory()),
        total_swap_bytes: Some(sys.total_swap()),
        cpu_static_info: cpu_static_info_opt,
        country_code: country_opt,
    }
}
