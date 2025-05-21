use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_file = "../proto/server.proto";
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    let mut builder = tonic_build::configure(); // Create the builder

    // Configure file descriptor set path
    builder = builder.file_descriptor_set_path(out_dir.join("agent_service_descriptor.bin"));

    // Compile the protos
    // The compile method takes `mut self`, so `builder` needs to be mutable here.
    // It consumes `self` and returns `Result<Self, Error>`.
    // We are interested in the Result, not reassigning builder if it's the last step in its usage for compilation.
    builder.compile_protos(&[proto_file], &["../proto"])?;

    // Tell cargo to re-run this build script if the proto file changes.
    println!("cargo:rerun-if-changed={}", proto_file);

    Ok(())
}