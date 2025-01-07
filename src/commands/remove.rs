use std::fs;
use std::io;
use std::path::Path;

/// Removes all files within the specified output directory.
/// If `all` is true, it will remove the entire `.leli` directory.
///
/// # Arguments
///
/// * `output_folder` - The path to the output folder to be removed.
/// * `all` - Whether to remove the entire `.leli` directory.
pub fn remove_output_folder(output_folder: &str, all: bool) -> io::Result<()> {
    let path = Path::new(output_folder);

    if all {
        let home_dir = dirs::home_dir().expect("Could not determine home directory");
        let leli_root = home_dir.join(".leli");
        println!("Removing all projects within: {}", leli_root.display());

        if leli_root.exists() {
            fs::remove_dir_all(&leli_root)?;
            println!("Successfully removed all projects.");
        } else {
            println!("No projects found to remove.");
        }
    } else {
        if path.exists() {
            fs::remove_dir_all(path)?;
            println!("Successfully removed: {}", path.display());
        } else {
            println!("Output folder does not exist: {}", path.display());
        }
    }

    Ok(())
}
