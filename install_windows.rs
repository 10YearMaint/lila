use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() {
    // Figure out where Cargo built your binary
    let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".into());
    let release_dir = format!("{}/release", target_dir);

    // Path to the compiled binary (Release mode)
    let binary_path = PathBuf::from(&release_dir).join("lila.exe");
    // Path to the sqlite3.dll that was placed next to your exe by build.rs
    let dll_path = PathBuf::from(&release_dir).join("sqlite3.dll");

    println!("The binary is located at: {}", binary_path.display());

    print!("Do you want to make 'lila.exe' available system-wide (or user-wide)? (y/N): ");
    io::stdout().flush().unwrap();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    let answer = answer.trim().to_lowercase();

    if answer == "y" || answer == "yes" {
        // Prompt for the install destination.
        println!(
            "Please enter the path where you'd like to install 'lila.exe'.\n\
             It should be a folder included in your PATH (e.g., \
             C:\\Users\\<user>\\AppData\\Local\\Microsoft\\WindowsApps)."
        );
        print!("Destination directory (press Enter to cancel): ");
        io::stdout().flush().unwrap();

        let mut dest_dir_input = String::new();
        io::stdin().read_line(&mut dest_dir_input).unwrap();
        let dest_dir_input = dest_dir_input.trim();

        if dest_dir_input.is_empty() {
            println!("No directory specified. Aborting install.");
            return;
        }

        let dest_dir = PathBuf::from(dest_dir_input);
        if !dest_dir.exists() {
            println!("Destination directory does not exist. Attempting to create it...");
            if let Err(e) = fs::create_dir_all(&dest_dir) {
                eprintln!("Failed to create directory {}: {}", dest_dir.display(), e);
                return;
            }
        }

        let dest_path = dest_dir.join("lila.exe");

        // Attempt to copy the exe
        match fs::copy(&binary_path, &dest_path) {
            Ok(_) => {
                println!("'lila.exe' is now installed at {}", dest_path.display());

                // Also attempt to copy the DLL
                let dll_dest_path = dest_dir.join("sqlite3.dll");
                match fs::copy(&dll_path, &dll_dest_path) {
                    Ok(_) => {
                        println!("'sqlite3.dll' is now installed at {}", dll_dest_path.display());
                        println!("If that directory is on your PATH, you can now type 'lila' anywhere.");
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to copy 'sqlite3.dll' to {}: {}",
                            dll_dest_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => eprintln!("Failed to copy 'lila.exe' to {}: {}", dest_path.display(), e),
        }
    } else {
        println!("Installation cancelled; 'lila.exe' was not copied.");
    }
}
