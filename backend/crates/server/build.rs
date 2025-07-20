use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // --- Add locale copying logic ---
    let dest_path = out_dir.join("../../.."); // Navigate to the target/debug or target/release directory

    // Define the source locales directory, relative to the project root
    let source_path = Path::new("../../../locales");

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
