use std::net::IpAddr;
use sysinfo::{Networks, System}; // Added System
use crate::agent_service::StaticSystemInfo; // Import the protobuf message

// Helper function to collect public IP addresses
pub fn collect_public_ip_addresses() -> Vec<String> {
    let mut public_ips = Vec::new();
    let networks = Networks::new_with_refreshed_list();

    for (_if_name, network_data) in networks.iter() {
        for ip_network in network_data.ip_networks() {
            let ip_addr = ip_network.addr;
            if ip_addr.is_loopback() || ip_addr.is_multicast() {
                continue;
            }

            match ip_addr {
                IpAddr::V4(ipv4_addr) => {
                    if ipv4_addr.is_link_local() || // 169.254.0.0/16
                       ipv4_addr.is_private() || // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                       ipv4_addr.is_documentation() ||
                       ipv4_addr.is_broadcast() ||
                       ipv4_addr.is_unspecified() // 0.0.0.0
                    {
                        continue;
                    }
                    public_ips.push(ipv4_addr.to_string());
                }
                IpAddr::V6(ipv6_addr) => {
                    let segments = ipv6_addr.segments();
                    if !(ipv6_addr.is_unspecified() ||
                        ipv6_addr.is_loopback() ||
                        ipv6_addr.is_multicast() ||
                        // Link-local (fe80::/10)
                        (segments[0] & 0xffc0 == 0xfe80) ||
                        // Unique Local Addresses (fc00::/7)
                        (segments[0] & 0xfe00 == 0xfc00) ||
                        // Documentation (2001:db8::/32)
                        (segments[0] == 0x2001 && segments[1] == 0x0db8))
                    {
                        public_ips.push(ipv6_addr.to_string());
                    }
                }
            }
        }
    }
    // Sort and dedup for consistent order and uniqueness
    public_ips.sort_unstable();
    public_ips.dedup();
    public_ips
}

// Helper function to collect static system information
pub fn collect_static_system_info() -> StaticSystemInfo {
    let mut sys = System::new_all();
    sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything()); // Refresh CPU info for brand
    // sys.refresh_all(); // Refresh all system information

    let mut architecture = System::cpu_arch();
    if architecture.is_empty() {
        architecture = "Unknown".to_string();
    }
    // Get the brand of the first CPU, or "Unknown" if no CPUs are found or brand is empty.
    let cpu_model = sys.cpus().first().map_or_else(
        || "Unknown".to_string(),
        |cpu| {
            let brand = cpu.brand();
            if brand.is_empty() { "Unknown".to_string() } else { brand.to_string() }
        }
    );
    let os_family = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());

    StaticSystemInfo {
        architecture,
        cpu_model,
        os_family,
        os_version,
        kernel_version,
        hostname,
    }
}