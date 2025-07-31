use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task;
use tokio::time::interval;
use tracing::{error, info};

use crate::server::config::ServerConfig;
use crate::version::VERSION;

pub struct SelfUpdateService {
    config: Arc<ServerConfig>,
    shutdown_rx: watch::Receiver<()>,
}

impl SelfUpdateService {
    pub fn new(config: Arc<ServerConfig>, shutdown_rx: watch::Receiver<()>) -> Self {
        Self { config, shutdown_rx }
    }

    pub async fn start_periodic_check(mut self) {
        // Run once on startup, then periodically
        self.run_update_check().await;

        let mut interval = interval(Duration::from_secs(4 * 60 * 60)); // Check every 4 hours
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.run_update_check().await;
                },
                _ = self.shutdown_rx.changed() => {
                    info!("Self-update service shutting down.");
                    break;
                }
            }
        }
    }

    async fn run_update_check(&self) {
        if self.config.is_in_container {
            info!("Running in a container, skipping self-update check.");
            return;
        }

        info!("Checking for new server version...");

        // This is a blocking operation, so we spawn it in a blocking task.
        let update_result = task::spawn_blocking(move || {
            let status = self_update::backends::github::Update::configure()
                .repo_owner("moonheart")
                .repo_name("NodeNexus")
                .bin_name("nodenexus-server")
                .show_download_progress(true)
                .current_version(VERSION)
                .build();

            let updater = match status {
                Ok(updater) => updater,
                Err(e) => {
                    // We can't use tracing::error here as we are in a sync context.
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
                    info!("Update successful! New version: {}. Server will restart.", status.version());
                    std::process::exit(0);
                } else {
                    info!("Server is up to date. Current version: {}", status.version());
                }
            },
            Ok(Err(e)) => {
                error!("Update check failed: {}", e);
            },
            Err(e) => {
                error!("Update task panicked: {}", e);
            }
        }
    }
}