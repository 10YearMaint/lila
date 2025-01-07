use clap::Parser;
use dirs::home_dir;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

mod commands;
mod schema;
mod utils;

use commands::{extract::*, remove::*, save::*, translate::*, Args, Commands};
use utils::{ensure_pandoc_installed, process_protocol_aimm};

/// Appends or updates `CODELITERAT_OUTPUT_PATH` in a local `.env` file
/// without including subfolders (like `.app` or `doc`).
fn update_dotenv(root_folder: &Path) -> io::Result<()> {
    let dotenv_path = Path::new(".env");
    let output_str = root_folder.to_string_lossy();
    let line_to_write = format!("CODELITERAT_OUTPUT_PATH={}", output_str);

    // If .env doesn't exist, create it
    if !dotenv_path.exists() {
        let mut file = File::create(dotenv_path)?;
        writeln!(file, "# Leli environment settings")?;
        writeln!(file, "{}", line_to_write)?;
        return Ok(());
    }

    // If .env exists, see if CODELITERAT_OUTPUT_PATH is already present
    let content = fs::read_to_string(dotenv_path)?;
    let mut lines: Vec<&str> = content.lines().collect();
    let mut found = false;

    for line in &mut lines {
        if line.trim_start().starts_with("CODELITERAT_OUTPUT_PATH=") {
            *line = &line_to_write;
            found = true;
            break;
        }
    }

    if !found {
        lines.push("# Leli environment settings (appended)");
        lines.push(&line_to_write);
    }

    // Rewrite .env
    let mut file = File::create(dotenv_path)?;
    for line in lines {
        writeln!(file, "{}", line)?;
    }

    Ok(())
}

fn main() {
    let args = Args::parse();

    // Default root folder => ~/.leli/<project_name>
    let home = home_dir().expect("Could not determine the home directory");
    let leli_root = home.join(".leli");
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let project_name = current_dir
        .file_name()
        .unwrap_or_else(|| OsStr::new("default"))
        .to_string_lossy()
        .to_string();
    let default_root = leli_root.join(&project_name);

    match &args.command {
        // ------------------ Extract Command ------------------
        Commands::Extract {
            file,
            folder,
            output,
            protocol,
        } => {
            let root_folder = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_root.clone());

            let app_folder = root_folder.join(".app");
            fs::create_dir_all(&app_folder)
                .unwrap_or_else(|e| panic!("Could not create .app folder: {}", e));

            // Update .env with just the root folder
            if let Err(e) = update_dotenv(&root_folder) {
                eprintln!("Warning: Could not update .env: {}", e);
            }

            // Extraction logic
            if let Some(file) = file {
                match extract_code_from_markdown(file) {
                    Ok(Ok(extracted_code)) => {
                        for (filename, code) in extracted_code {
                            let output_path = app_folder.join(filename);
                            if let Some(parent) = output_path.parent() {
                                fs::create_dir_all(parent).unwrap();
                            }
                            let mut output_file = File::create(&output_path).unwrap();
                            output_file.write_all(code.as_bytes()).unwrap();
                            println!("Code extracted to {}", output_path.display());
                        }
                    }
                    Ok(Err(_)) => {
                        let output_path = app_folder.join(Path::new(file).file_name().unwrap());
                        fs::copy(file, &output_path).unwrap();
                        println!("Copied file to {}", output_path.display());
                    }
                    Err(e) => eprintln!("Error extracting code: {}", e),
                }
            } else if let Some(folder) = folder {
                if let Err(e) = extract_code_from_folder(folder, &app_folder.to_string_lossy()) {
                    eprintln!("Error extracting code: {}", e);
                }
            }

            // Protocol check
            if let Some(protocol) = protocol {
                if protocol == "AImM" {
                    println!("Protocol AImM detected. Combining folders...");
                    if let Err(e) = process_protocol_aimm(&app_folder) {
                        eprintln!("Error processing protocol AImM: {}", e);
                    }
                } else {
                    println!("Protocol detected but not AImM.");
                }
            } else {
                println!("No protocol specified.");
            }
        }

        // ------------------ Translate Command ------------------
        Commands::Translate {
            folder,
            output,
            css,
            mermaid,
        } => {
            let root_folder = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_root.clone());

            let doc_folder = root_folder.join("doc");

            fs::create_dir_all(&doc_folder)
                .unwrap_or_else(|e| panic!("Could not create doc folder: {}", e));

            // Update .env with just the root folder
            if let Err(e) = update_dotenv(&root_folder) {
                eprintln!("Warning: Could not update .env: {}", e);
            }

            let css_path = css
                .clone()
                .unwrap_or_else(|| "src/css/style.css".to_string());
            let mermaid_path = mermaid
                .clone()
                .unwrap_or_else(|| "src/js/mermaid.min.js".to_string());

            if !ensure_pandoc_installed() {
                eprintln!("Pandoc is not installed. Please install Pandoc to use this tool.");
                std::process::exit(1);
            }

            if let Err(e) = translate_markdown_folder(
                &folder,
                &doc_folder.to_string_lossy(),
                &css_path,
                &mermaid_path,
            ) {
                eprintln!("Error translating markdown: {}", e);
            }
        }

        // ------------------ Save Command ------------------
        Commands::Save { file, db } => {
            let created_files =
                fs::read_to_string(file).expect("Unable to read created files list");
            let html_files: Vec<String> = created_files.lines().map(|s| s.to_string()).collect();
            let mut conn = establish_connection(db);
            if let Err(e) = save_html_metadata_to_db(&html_files, &mut conn, db) {
                eprintln!("Error saving HTML metadata to database: {}", e);
            }
        }

        // ------------------ Rm Command ------------------
        Commands::Rm { all, output } => {
            let root_folder = output
                .as_ref()
                .map(|path| PathBuf::from(path))
                .unwrap_or_else(|| default_root.clone());

            if let Err(e) = remove_output_folder(&root_folder.to_string_lossy(), *all) {
                eprintln!("Error removing project files: {}", e);
            }
        }
    }
}
