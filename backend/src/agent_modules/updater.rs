use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use std::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use std::env;
use crate::version::VERSION;

#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

const GITHUB_REPO: &str = "moonheart/NodeNexus";

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize, Debug)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Fetches the latest release information from GitHub.
async fn get_latest_github_release() -> Result<GitHubRelease, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    
    info!(url = %url, "Fetching latest release from GitHub");

    let response = client
        .get(&url)
        .header("User-Agent", "node-nexus-agent-updater")
        .send()
        .await?
        .error_for_status()?;

    let release: GitHubRelease = response.json().await?;
    Ok(release)
}

/// Downloads a file from a URL to a temporary path.
async fn download_asset(asset_url: &str, temp_path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(url = %asset_url, path = ?temp_path, "Downloading asset");
    
    let response = reqwest::get(asset_url).await?.error_for_status()?;
    
    let bytes = response.bytes().await?;
    
    let mut file = File::create(temp_path).await?;
    file.write_all(&bytes).await?;
    
    info!("Asset downloaded successfully.");
    Ok(())
}


/// Checks if the agent is likely running under systemd by checking for the INVOCATION_ID env var.
fn is_running_under_systemd() -> bool {
    env::var("INVOCATION_ID").is_ok()
}

/// Checks if the agent is likely running under launchd by checking for the LAUNCHD_SOCKET env var.
fn is_running_under_launchd() -> bool {
    env::var("LAUNCHD_SOCKET").is_ok()
}

async fn replace_and_restart(new_binary_path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let current_exe = env::current_exe()?;
    info!(current = ?current_exe, new = ?new_binary_path, "Replacing current executable");

    #[cfg(unix)]
    {
        // On Unix-like systems, we replace the binary file first.
        fs::rename(new_binary_path, &current_exe)?;
        info!("Binary replaced successfully.");

        if let Ok(service_name) = env::var("NEXUS_AGENT_SERVICE_NAME") {
            if is_running_under_systemd() {
                info!("Restarting via systemctl for service: {}", service_name);
                let restart_status = Command::new("systemctl")
                    .arg("restart")
                    .arg(&service_name)
                    .status()
                    .await?;
                
                if !restart_status.success() {
                    let msg = format!("'systemctl restart {}' failed with status: {}. The agent might need manual intervention.", service_name, restart_status);
                    error!("{}", msg);
                    return Err(msg.into());
                }
                
                info!("systemd service '{}' restarted successfully. Exiting old process.", service_name);
                std::process::exit(0);
            } else if is_running_under_launchd() {
                info!("Restarting via launchctl for service: {}", service_name);
                
                // Stop the service. Failure is not critical, it might already be stopped.
                let stop_status = Command::new("launchctl").arg("stop").arg(&service_name).status().await?;
                if !stop_status.success() {
                    warn!("'launchctl stop {}' failed with status: {}. This might be okay if the service is already stopped.", service_name, stop_status);
                }

                // Start the service
                let start_status = Command::new("launchctl").arg("start").arg(&service_name).status().await?;
                if !start_status.success() {
                    let msg = format!("'launchctl start {}' failed with status: {}. The agent might need manual intervention.", service_name, start_status);
                    error!("{}", msg);
                    return Err(msg.into());
                }

                info!("launchd service '{}' started successfully. Exiting old process.", service_name);
                std::process::exit(0);
            } else {
                warn!("NEXUS_AGENT_SERVICE_NAME is set, but no known service manager (systemd, launchd) was detected. Falling back to exec.");
            }
        }

        info!("Restarting as a standalone process via exec...");
        let err = std::process::Command::new(&current_exe).exec();
        Err(Box::new(err)) // This is only reached if exec fails
    }

    #[cfg(windows)]
    {
        let old_exe_bak = current_exe.with_extension("bak");
        
        if let Err(e) = fs::rename(&current_exe, &old_exe_bak) {
            warn!(error = %e, "Failed to rename running executable. Update cannot proceed.");
            return Err(e.into());
        }

        if let Err(e) = fs::copy(new_binary_path, &current_exe) {
            error!(error = %e, "Failed to copy new binary into place. Attempting to roll back.");
            if let Err(rb_err) = fs::rename(&old_exe_bak, &current_exe) {
                error!(error = %rb_err, "CRITICAL: Failed to restore original executable. The agent is in a broken state.");
            }
            return Err(e.into());
        }
        
        if let Err(e) = fs::remove_file(new_binary_path) {
            warn!(error = %e, "Failed to remove temporary update file.");
        }

        if let Ok(service_name) = env::var("NEXUS_AGENT_SERVICE_NAME") {
            info!("Attempting to restart service '{}' via SCM...", service_name);

            let stop_status = Command::new("sc.exe").arg("stop").arg(&service_name).status().await?;
            if !stop_status.success() {
                warn!("'sc.exe stop {}' failed with status: {}. The service might not have stopped cleanly.", service_name, stop_status);
            } else {
                info!("Service '{}' stopped successfully.", service_name);
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            let start_status = Command::new("sc.exe").arg("start").arg(&service_name).status().await?;
            if !start_status.success() {
                let msg = format!("'sc.exe start {}' failed with status: {}. The agent might need manual intervention.", service_name, start_status);
                error!("{}", msg);
                return Err(msg.into());
            }
            
            info!("Service '{}' started successfully. Exiting old process.", service_name);
            std::process::exit(0);
        } else {
            info!("Spawning command to restart agent and exiting current process...");
            const DETACHED_PROCESS: u32 = 0x00000008;
            std::process::Command::new(&current_exe)
                .creation_flags(DETACHED_PROCESS)
                .spawn()?;
            std::process::exit(0);
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err("Auto-update not supported on this platform.".into())
    }
}


/// Handles the agent update check process.
/// It uses a lock to ensure only one update process can run at a time.
pub async fn handle_update_check(update_lock: Arc<Mutex<()>>) {
    info!("Update check triggered. Attempting to acquire update lock...");

    let Ok(_lock_guard) = update_lock.try_lock() else {
        warn!("Update process is already running. Skipping this trigger.");
        return;
    };

    info!("Update lock acquired. Starting update process...");

    let current_version = VERSION;
    info!(version = current_version, "Current agent version");

    match get_latest_github_release().await {
        Ok(latest_release) => {
            info!(latest_version = %latest_release.tag_name, "Found latest GitHub release.");
            let latest_version_normalized = latest_release.tag_name.trim_start_matches('v');

            if latest_version_normalized != current_version.trim_start_matches('v') {
                info!("New version available! Current: {}, Latest: {}", current_version, latest_version_normalized);
                
                let arch = match std::env::consts::ARCH {
                    "x86_64" => "amd64",
                    "aarch64" => "arm64",
                    other => {
                        warn!(arch = other, "Unsupported architecture for auto-update.");
                        return;
                    }
                };
                let os = std::env::consts::OS;
                
                let mut target_asset_name = format!("agent-{}-{}", os, arch);
                if os == "windows" {
                    target_asset_name.push_str(".exe");
                }

                info!(asset_name = %target_asset_name, "Looking for release asset");

                if let Some(asset_to_download) = latest_release.assets.iter().find(|a| a.name == target_asset_name) {
                    let temp_dir = std::env::temp_dir();
                    let temp_file_path = temp_dir.join(&asset_to_download.name);

                    match download_asset(&asset_to_download.browser_download_url, &temp_file_path).await {
                        Ok(_) => {
                            info!(path = ?temp_file_path, "New version downloaded successfully.");
                            
                            // Set executable permissions on Unix-like systems
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                info!("Setting executable permissions on downloaded file.");
                                if let Err(e) = fs::set_permissions(&temp_file_path, fs::Permissions::from_mode(0o755)) {
                                    error!(error = %e, "Failed to set executable permissions.");
                                    return; // Can't proceed without executable permissions
                                }
                            }

                            // Run health check on the new binary
                            info!("Running health check on the new binary...");
                            match Command::new(&temp_file_path).arg("--health-check").status().await {
                                Ok(status) if status.success() => {
                                    info!("Health check passed. Proceeding with replacement.");
                                    if let Err(e) = replace_and_restart(&temp_file_path).await {
                                        error!(error = %e, "Failed to replace and restart the agent.");
                                    }
                                    // If replace_and_restart succeeds (on unix), this code is not reached.
                                    // On windows, it exits.
                                }
                                Ok(status) => {
                                    error!(exit_code = ?status.code(), "Health check failed with non-zero exit code.");
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to execute health check command.");
                                }
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to download new version.");
                        }
                    }
                } else {
                    warn!("Could not find a matching release asset for this platform.");
                }

            } else {
                info!("Agent is already up to date.");
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to check for new version on GitHub.");
        }
    }

    info!("Update process finished. Releasing lock.");
}