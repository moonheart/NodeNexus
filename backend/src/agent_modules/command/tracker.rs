use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tracing::{info, warn, debug};

// The tracker now holds a sender part of a one-shot channel.
// Sending a message on this channel signals the command's managing task to terminate it.
#[derive(Debug, Clone)]
pub struct RunningCommandsTracker {
    commands: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

impl RunningCommandsTracker {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_command(&self, command_id: String, term_tx: oneshot::Sender<()>) {
        let mut commands_guard = self.commands.lock().unwrap();
        if commands_guard.insert(command_id.clone(), term_tx).is_some() {
            // This case (replacing an existing command) should ideally not happen
            // if command IDs are unique.
            warn!(command_id = %command_id, "Replaced an existing command in tracker. This may indicate a command ID collision.");
        }
        info!(command_id = %command_id, "Added command to tracker.");
    }

    // This function is called by the command's managing task upon completion.
    pub fn remove_command(&self, command_id: &str) {
        let mut commands_guard = self.commands.lock().unwrap();
        if commands_guard.remove(command_id).is_some() {
            info!(command_id = %command_id, "Removed command from tracker on completion.");
        }
    }

    // This function is called by the termination handler.
    pub fn signal_termination(&self, command_id: &str) -> Result<(), &'static str> {
        // Remove the sender from the map to prevent multiple signals.
        // The receiving end of the oneshot channel will be dropped when the command task finishes,
        // so sending might fail if the command has already completed. This is expected.
        if let Some(term_tx) = self.commands.lock().unwrap().remove(command_id) {
            if term_tx.send(()).is_ok() {
                info!(command_id = %command_id, "Termination signal sent.");
            } else {
                debug!(command_id = %command_id, "Command already finished, no termination signal needed.");
            }
            Ok(())
        } else {
            // This can happen if a termination signal is sent for a command that has already completed
            // and been removed from the tracker, or was already terminated.
            debug!(command_id = %command_id, "Command not found for termination (already terminated or completed).");
            Err("Command not found or already terminated.")
        }
    }
}

impl Default for RunningCommandsTracker {
    fn default() -> Self {
        Self::new()
    }
}