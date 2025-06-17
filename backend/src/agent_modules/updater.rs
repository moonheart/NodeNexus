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


fn replace_and_restart(new_binary_path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let current_exe = env::current_exe()?;
    info!(current = ?current_exe, new = ?new_binary_path, "Replacing current executable");

    // This is a simplified version. A robust implementation would handle permissions,
    // potential rollback, and different OS-specific edge cases.
    // The `self_update` crate is a good reference for a production-grade solution.

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // On Unix, we can replace the binary and then use exec to restart.
        fs::rename(new_binary_path, &current_exe)?;
        info!("Binary replaced. Restarting agent via exec...");
        // The error from exec is only returned if exec fails.
        let err = std::process::Command::new(&current_exe).exec();
        return Err(Box::new(err));
    }

    #[cfg(windows)]
    {
        // On Windows, we can't replace a running executable.
        // A common strategy is to use a helper script.
        // For simplicity here, we'll try a rename, which is likely to fail but demonstrates the idea.
        // A production solution would use a detached process with a batch script.
        let old_exe_bak = current_exe.with_extension("bak");
        
        // Try to move the current executable to a .bak file
        if fs::rename(&current_exe, &old_exe_bak).is_err() {
            // This is expected to fail on Windows if the process is running.
            // We log it and proceed to try and write a helper script.
            warn!("Could not rename running executable. This is expected on Windows. Will try helper script approach.");
        }

        // Move the new binary into place
        fs::rename(new_binary_path, &current_exe)?;

        info!("Spawning command to restart agent and exiting current process...");
        std::process::Command::new(&current_exe).spawn()?;
        std::process::exit(0);
    }

    #[cfg(not(any(unix, windows)))]
    {
        return Err("Auto-update not supported on this platform.".into());
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
                                    if let Err(e) = replace_and_restart(&temp_file_path) {
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