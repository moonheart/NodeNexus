use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::process::Child;
// Potentially, if using Uuid for internal tracking before converting to string for map:
// use uuid::Uuid;

#[derive(Debug)]
pub struct ChildProcessHandle {
    pub child: Option<Child>, // Changed to Option to allow taking ownership
    pub pid: Option<u32>,     // Store PID for logging/reference
}

// Key is child_command_id (String) as per protobuf
#[derive(Debug, Clone)]
pub struct RunningCommandsTracker {
    commands: Arc<Mutex<HashMap<String, Arc<Mutex<ChildProcessHandle>>>>>,
}

impl RunningCommandsTracker {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_command(&self, command_id: String, handle: ChildProcessHandle) -> Result<(), String> {
        let mut commands_guard = self.commands.lock().unwrap();
        if commands_guard.contains_key(&command_id) {
            return Err(format!("Command ID {} already exists in tracker.", command_id));
        }
        commands_guard.insert(command_id.clone(), Arc::new(Mutex::new(handle)));
        println!("[Tracker] Added command: {}", command_id);
        Ok(())
    }

    pub async fn remove_command(&self, command_id: &str) -> Option<Arc<Mutex<ChildProcessHandle>>> {
        let mut commands_guard = self.commands.lock().unwrap();
        let removed = commands_guard.remove(command_id);
        if removed.is_some() {
            println!("[Tracker] Removed command: {}", command_id);
        } else {
            println!("[Tracker] Attempted to remove non-existent command: {}", command_id);
        }
        removed
    }

    pub fn get_command_handle(&self, command_id: &str) -> Option<Arc<Mutex<ChildProcessHandle>>> {
        let commands_guard = self.commands.lock().unwrap();
        commands_guard.get(command_id).cloned()
    }

    // Optional: A method to try to kill a command
    pub async fn kill_command(&self, command_id: &str) -> Result<String, String> {
        let handle_arc = match self.get_command_handle(command_id) {
            Some(h) => h,
            None => {
                eprintln!("[Tracker] Command {} not found for killing.", command_id);
                return Err(format!("Command {} not found for killing.", command_id));
            }
        };

        // Take the child from the handle to release the MutexGuard before .await
        let mut child_to_kill = {
            let mut guard = handle_arc.lock().unwrap();
            guard.child.take()
        }; // MutexGuard is dropped here

        if let Some(mut child) = child_to_kill {
            match child.kill().await {
                Ok(_) => {
                    println!("[Tracker] Kill signal sent to command: {}", command_id);
                    Ok(format!("Kill signal sent to command {}", command_id))
                }
                Err(e) => {
                    eprintln!("[Tracker] Failed to send kill signal to command {}: {}", command_id, e);
                    Err(format!("Failed to kill command {}: {}", command_id, e))
                }
            }
        } else {
            let msg = format!("Command {} was already terminated or handle was empty.", command_id);
            eprintln!("[Tracker] {}", msg);
            // Arguably, this could be a success if the goal is "make sure it's not running"
            Ok(msg)
        }
    }
}

impl Default for RunningCommandsTracker {
    fn default() -> Self {
        Self::new()
    }
}