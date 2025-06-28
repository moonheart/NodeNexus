use crate::agent_service::StaticSystemInfo;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::net::IpAddr;
use std::str::FromStr; // For IpAddr::from_str
use std::time::Duration;
use sysinfo::System;
use tracing::{debug, error, info, warn};

const CF_TRACE_ENDPOINTS: &[&str] = &[
    "https://cloudflare.com/cdn-cgi/trace",
    "https://blog.cloudflare.com/cdn-cgi/trace",
    "https://developers.cloudflare.com/cdn-cgi/trace",
];

// User agent similar to what a browser might send
const CUSTOM_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.114 Safari/537.36";

static HTTP_CLIENT_V4: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(5)) // Reduced timeout
        .connect_timeout(Duration::from_secs(3)) // Reduced connect timeout
        .local_address("0.0.0.0".parse::<std::net::IpAddr>().ok())
        .user_agent(CUSTOM_USER_AGENT)
        .build()
        .unwrap_or_else(|e| {
            error!(error = %e, "Failed to create IPv4 HTTP client, falling back to default.");
            Client::new()
        })
});

static HTTP_CLIENT_V6: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(3))
        .local_address("::".parse::<std::net::IpAddr>().ok())
        .user_agent(CUSTOM_USER_AGENT)
        .build()
        .unwrap_or_else(|e| {
            error!(error = %e, "Failed to create IPv6 HTTP client, falling back to default.");
            Client::new()
        })
});

async fn fetch_ip_and_loc_from_endpoints(
    endpoints: &'static [&'static str],
    client: &Lazy<Client>,
    ip_version_is_v6: bool,
) -> Option<(String, Option<String>)> {
    for endpoint in endpoints {
        debug!(endpoint = %endpoint, "Attempting to fetch IP and Loc.");
        match client.get(*endpoint).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => {
                            let mut ip_address: Option<String> = None;
                            let mut loc: Option<String> = None;
                            for line in body.lines() {
                                if line.starts_with("ip=") {
                                    let ip_str = line.trim_start_matches("ip=").trim();
                                    if let Ok(parsed_ip) = IpAddr::from_str(ip_str) {
                                        if (ip_version_is_v6 && parsed_ip.is_ipv6())
                                            || (!ip_version_is_v6 && parsed_ip.is_ipv4())
                                        {
                                            ip_address = Some(ip_str.to_string());
                                        } else {
                                            debug!(endpoint = %endpoint, ip = %ip_str, "Mismatched IP type from endpoint.");
                                        }
                                    } else {
                                        warn!(ip_str = %ip_str, endpoint = %endpoint, "Failed to parse IP string from endpoint.");
                                    }
                                } else if line.starts_with("loc=") {
                                    loc = Some(line.trim_start_matches("loc=").trim().to_string());
                                }
                            }

                            if let Some(ip) = ip_address {
                                info!(ip = %ip, loc = ?loc, endpoint = %endpoint, "Successfully fetched IP and Loc.");
                                return Some((ip, loc));
                            } else {
                                warn!(endpoint = %endpoint, response_prefix = %body.chars().take(100).collect::<String>(), "'ip=' field (matching version) not found or invalid in response.");
                            }
                        }
                        Err(e) => {
                            warn!(endpoint = %endpoint, error = %e, "Failed to read response body.");
                        }
                    }
                } else {
                    warn!(endpoint = %endpoint, status = %resp.status(), "Request failed.");
                }
            }
            Err(e) => {
                if e.is_connect() || e.is_timeout() {
                    warn!(endpoint = %endpoint, error = %e, "Connection error. Might indicate no route for this IP version.");
                } else {
                    warn!(endpoint = %endpoint, error = %e, "Failed to send request.");
                }
            }
        }
    }
    None
}

pub async fn collect_public_ip_addresses() -> (Vec<String>, Option<String>) {
    info!("Starting public IP address and country code collection...");

    let ipv4_future = fetch_ip_and_loc_from_endpoints(CF_TRACE_ENDPOINTS, &HTTP_CLIENT_V4, false);
    let ipv6_future = fetch_ip_and_loc_from_endpoints(CF_TRACE_ENDPOINTS, &HTTP_CLIENT_V6, true);

    let (ipv4_data, ipv6_data) = tokio::join!(ipv4_future, ipv6_future);

    let mut public_ips = Vec::new();
    let mut country_code: Option<String> = None;

    if let Some((ip4, loc4)) = ipv4_data {
        info!(ip = %ip4, loc = ?loc4, "Collected IPv4 address.");
        public_ips.push(ip4);
        if country_code.is_none() {
            // Prioritize loc from IPv4 if available
            country_code = loc4;
        }
    } else {
        warn!("Failed to collect IPv4 address from all providers.");
    }

    if let Some((ip6, loc6)) = ipv6_data {
        info!(ip = %ip6, loc = ?loc6, "Collected IPv6 address.");
        public_ips.push(ip6);
        if country_code.is_none() {
            // If no loc from IPv4, use loc from IPv6
            country_code = loc6;
        }
    } else {
        warn!("Failed to collect IPv6 address from all providers.");
    }

    if public_ips.is_empty() {
        warn!("Failed to collect any public IP address.");
    }
    if country_code.is_none() {
        warn!("Failed to collect country code.");
    }

    (public_ips, country_code)
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
            if brand.is_empty() {
                "Unknown".to_string()
            } else {
                brand.to_string()
            }
        },
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
