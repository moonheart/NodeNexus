use crate::version::VERSION;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use tracing::{error, info, warn};

/// Handles the agent update check process.
/// It uses a lock to ensure only one update process can run at a time.
pub async fn handle_update_check(update_lock: Arc<Mutex<()>>) {
    info!("Update check triggered. Attempting to acquire update lock...");

    let Ok(_lock_guard) = update_lock.try_lock() else {
        warn!("Update process is already running. Skipping this trigger.");
        return;
    };

    info!("Update lock acquired. Starting update process...");

    // This is a blocking operation, so we spawn it in a blocking task.
    let update_result = task::spawn_blocking(move || {
        let status = self_update::backends::github::Update::configure()
            .repo_owner("moonheart")
            .repo_name("NodeNexus")
            .bin_name("nodenexus-agent") // Make sure this matches the release asset name
            .show_download_progress(true)
            .current_version(VERSION)
            .build();

        let updater = match status {
            Ok(updater) => updater,
            Err(e) => {
                println!("[ERROR] Failed to build updater: {}", e);
                return Err(e.to_string());
            }
        };

        match updater.update() {
            Ok(status) => Ok(status),
            Err(e) => {
                println!("[ERROR] Update failed: {}", e);
                Err(e.to_string())
            }
        }
    }).await;

    match update_result {
        Ok(Ok(status)) => {
             if status.updated() {
                info!("Update successful! New version: {}. Agent will restart.", status.version());
                // The self_update crate handles the restart logic.
                // We exit here to let the new process take over.
                std::process::exit(0);
            } else {
                info!("Agent is up to date. Current version: {}", status.version());
            }
        },
        Ok(Err(e)) => {
            error!("Update check failed: {}", e);
        },
        Err(e) => {
            error!("Update task panicked: {}", e);
        }
    }
    
    info!("Update process finished. Releasing lock.");
}
