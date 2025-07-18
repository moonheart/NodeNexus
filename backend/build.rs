use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = [
        "./proto/common.proto",
        "./proto/handshake.proto",
        "./proto/config.proto",
        "./proto/metrics.proto",
        "./proto/docker.proto",
        "./proto/generic_metrics.proto",
        "./proto/command.proto",
        "./proto/pty.proto",
        "./proto/messages.proto",
        "./proto/service.proto",
        "./proto/batch_command.proto",
    ];
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    tonic_build::configure()
        .out_dir(out_dir.clone())
        .type_attribute(
            "agent_service.AgentConfig",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .field_attribute(
            "agent_service.AgentConfig.service_monitor_tasks",
            "#[serde(default)]",
        )
        .type_attribute(
            "agent_service.ServiceMonitorTask",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "agent_service.DiskUsage",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile_protos(&proto_files, &["./proto"])?;

    // Tell cargo to re-run this build script if any proto file changes.
    rerun(&proto_files);

    // --- Add locale copying logic ---
    let dest_path = out_dir.join("../../.."); // Navigate to the target/debug or target/release directory

    // Define the source locales directory, relative to the project root
    let source_path = Path::new("../locales");

    // Define the target locales directory
    let target_locales_path = dest_path.join("locales");

    // Create the target directory if it doesn't exist
    if !target_locales_path.exists() {
        fs::create_dir_all(&target_locales_path)?;
    }

    // Copy the contents of the source directory to the target directory
    for entry in fs::read_dir(source_path)? {
        let entry = entry?;
        let source_file = entry.path();
        let dest_file = target_locales_path.join(entry.file_name());
        fs::copy(&source_file, &dest_file)?;
    }

    println!("cargo:rerun-if-changed=../locales");
    // --- End of locale copying logic ---

    Ok(())
}

fn rerun(proto_files: &[&str]) {
    for proto_file in proto_files {
        println!("cargo:rerun-if-changed={proto_file}");
    }
}
