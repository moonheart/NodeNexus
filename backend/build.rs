use std::env;
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
        "./proto/heartbeat.proto",
        "./proto/messages.proto",
        "./proto/service.proto",
        "./proto/batch_command.proto",
    ];
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    tonic_build::configure()
        .out_dir(out_dir)
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
        .compile_protos(&proto_files, &["./proto"])?;

    // Tell cargo to re-run this build script if any proto file changes.
    rerun(&proto_files);

    Ok(())
}

fn rerun(proto_files: &[&str]) {
    for proto_file in proto_files {
        println!("cargo:rerun-if-changed={proto_file}");
    }
}
