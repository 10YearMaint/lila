use std::{env, fs, path::PathBuf};

fn main() {
    let dll_source = PathBuf::from("windows/sqlite3.dll");

    // figure out whether we’re building "debug" or "release" from the PROFILE var:
    let profile = env::var("PROFILE").expect("PROFILE not set by Cargo");

    // Cargo makes CARGO_MANIFEST_DIR point to the project root (where Cargo.toml is).
    let root_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Construct path to target/debug or target/release:
    let target_dir = root_dir.join("target").join(&profile);

    // The final location for the DLL (beside the .exe):
    let dll_dest = target_dir.join("sqlite3.dll");

    // Perform the copy:
    if let Err(e) = fs::copy(&dll_source, &dll_dest) {
        panic!("Failed to copy {:?} to {:?}: {}", dll_source, dll_dest, e);
    }

    println!("cargo:rustc-link-search=native=windows");
    println!("cargo:rustc-link-lib=static=sqlite3");
}
