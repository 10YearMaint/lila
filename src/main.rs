use clap::Parser;
use colored::Colorize;
use dirs::home_dir;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

mod commands;
mod schema;
mod utils;

use commands::chat::ChatArgs;
use commands::edit::{edit_format_code_in_folder, edit_format_code_in_markdown};
use commands::weave::{convert_file_to_markdown, convert_folder_to_markdown};
use commands::{
    chat::run_chat, init::init as run_init, remove::*, render::*, save::*, tangle::*, Args,
    Commands,
};
use utils::{env::ensure_pandoc_installed, utils::process_protocol_aimm};

fn main() {
    // 1) Parse CLI args
    let args = Args::parse();

    // 2) Load any existing .env (so that LILA_OUTPUT_PATH, BLACK_INSTALLED, etc. are in scope)
    dotenvy::dotenv().ok();

    // 3) The "project_name" logic is only for fallback if user doesn’t supply an --output path
    let home = home_dir().expect("Could not determine the home directory");
    let lila_root = home.join(".lila");
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let project_name = current_dir
        .file_name()
        .unwrap_or_else(|| OsStr::new("default"))
        .to_string_lossy()
        .to_string();
    let default_root = lila_root.join(&project_name);

    match &args.command {
        // ------------------ Init Command --------------------
        Commands::Init => {
            // We call the new init in commands/init
            if let Err(e) = run_init() {
                eprintln!("Error during init: {}", e);
            }
        }

        // ------------------ Tangle Command ------------------
        Commands::Tangle {
            file,
            folder,
            output,
            protocol,
        } => {
            // If user provided --output, use that, otherwise read from .env or fallback
            let root_folder = output
                .as_ref()
                .map(PathBuf::from)
                .or_else(|| {
                    // If .env has LILA_OUTPUT_PATH, use it
                    match std::env::var("LILA_OUTPUT_PATH") {
                        Ok(path) => Some(PathBuf::from(path)),
                        Err(_) => Some(default_root.clone()),
                    }
                })
                .unwrap_or(default_root.clone());

            let app_folder = root_folder.join(".app");
            fs::create_dir_all(&app_folder)
                .unwrap_or_else(|e| panic!("Could not create .app folder: {}", e));

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
                            let checkmark = "✔".green();
                            println!("{} Code extracted to {}", checkmark, output_path.display());
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

        // ------------------ Weave Command ------------------
        Commands::Weave {
            file,
            folder,
            output,
        } => {
            let root_folder = output
                .as_ref()
                .map(PathBuf::from)
                .or_else(|| match std::env::var("LILA_OUTPUT_PATH") {
                    Ok(path) => Some(PathBuf::from(path).join("doc")),
                    Err(_) => Some(default_root.join("doc")),
                })
                .unwrap_or(default_root.join("doc"));

            fs::create_dir_all(&root_folder)
                .unwrap_or_else(|e| panic!("Could not create output folder: {}", e));

            if let Some(file) = file {
                let input_path = PathBuf::from(file);
                if let Err(e) = convert_file_to_markdown(&input_path, &root_folder) {
                    eprintln!("Error converting file {}: {}", file, e);
                }
            } else if let Some(folder) = folder {
                if let Err(e) = convert_folder_to_markdown(folder, &root_folder.to_string_lossy()) {
                    eprintln!("Error converting folder {}: {}", folder, e);
                }
            } else {
                eprintln!("No file or folder provided for conversion.");
            }
        }

        // ------------------ Edit Command -------------------
        Commands::Edit { file, folder } => {
            // Decide whether we’re formatting a single file or an entire folder:
            if let Some(file) = file {
                // Single file
                if let Err(e) = edit_format_code_in_markdown(file) {
                    eprintln!("Error auto-formatting file {}: {}", file, e);
                }
            } else if let Some(folder) = folder {
                // Entire folder
                if let Err(e) = edit_format_code_in_folder(folder) {
                    eprintln!("Error auto-formatting folder {}: {}", folder, e);
                }
            } else {
                eprintln!("No file or folder provided for auto-formatting.");
            }
        }

        // ------------------ Render Command ------------------
        Commands::Render {
            folder,
            output,
            css,
            mermaid,
            disable_mermaid,
        } => {
            // Determine the root folder based on the disable_mermaid flag
            let root_folder = output
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_root.clone());

            // Choose between "doc" and "doc_pure" based on the flag
            let doc_folder = if *disable_mermaid {
                root_folder.join("doc_pure")
            } else {
                root_folder.join("doc")
            };

            // Create the appropriate documentation folder
            fs::create_dir_all(&doc_folder)
                .unwrap_or_else(|e| panic!("Could not create doc folder: {}", e));

            // Determine CSS and Mermaid paths
            let css_path = css
                .clone()
                .unwrap_or_else(|| "src/css/style.css".to_string());
            let mermaid_path = if *disable_mermaid {
                String::new() // Empty string signifies no Mermaid.js
            } else {
                mermaid
                    .clone()
                    .unwrap_or_else(|| "src/js/mermaid.min.js".to_string())
            };

            // Check if Pandoc is installed
            if !ensure_pandoc_installed() {
                eprintln!("Pandoc is not installed. Please install Pandoc to use this tool.");
                std::process::exit(1);
            }

            // Perform the translation
            if let Err(e) = translate_markdown_folder(
                &folder,
                &doc_folder.to_string_lossy(),
                &css_path,
                if *disable_mermaid {
                    None
                } else {
                    Some(&mermaid_path)
                },
            ) {
                eprintln!("Error translating markdown: {}", e);
            }
        }

        // ------------------ Save Command ------------------
        Commands::Save { db } => {
            // Define the doc_pure folder
            let doc_pure_folder = default_root.join("doc_pure");

            // Path to the created_html_files.txt
            let file_path = doc_pure_folder.join("created_html_files.txt");

            // Check if created_html_files.txt exists
            if !file_path.exists() {
                eprintln!(
                    "Error: '{}' does not exist. Please run the 'translate' command first.",
                    file_path.display()
                );
                std::process::exit(1);
            }

            // Read the created_html_files.txt
            let created_files =
                fs::read_to_string(&file_path).expect("Unable to read created_html_files.txt");

            let html_files: Vec<String> = created_files.lines().map(|s| s.to_string()).collect();

            // Determine the database path
            let db_path = db
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| doc_pure_folder.join("lila.db"));

            // Ensure the parent directory of the database exists
            if let Some(parent) = db_path.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|e| panic!("Could not create database directory: {}", e));
            }

            // Establish connection
            let mut conn = establish_connection(&db_path.to_string_lossy());

            // Save HTML metadata to the database
            if let Err(e) =
                save_html_metadata_to_db(&html_files, &mut conn, &db_path.to_string_lossy())
            {
                eprintln!("Error saving HTML metadata to database: {}", e);
            } else {
                println!(
                    "Successfully saved HTML metadata to '{}'",
                    db_path.display()
                );
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

        // ------------------ Chat Command ----------------
        Commands::Chat {
            cpu,
            tracing,
            verbose_prompt,
            prompt,
            temperature,
            top_p,
            seed,
            sample_len,
            model_id,
            model,
            revision,
            weight_file,
            tokenizer,
            quantized,
            repeat_penalty,
            repeat_last_n,
            dtype,
            no_db,
        } => {
            // Construct the ChatArgs struct and pass it to run_chat:
            let chat_args = ChatArgs {
                cpu: *cpu,
                tracing: *tracing,
                verbose_prompt: *verbose_prompt,
                prompt: prompt.clone(),
                temperature: *temperature,
                top_p: *top_p,
                seed: *seed,
                sample_len: *sample_len,
                model_id: model_id.clone(),
                model: model.clone(),
                revision: revision.clone(),
                weight_file: weight_file.clone(),
                tokenizer: tokenizer.clone(),
                quantized: *quantized,
                repeat_penalty: *repeat_penalty,
                repeat_last_n: *repeat_last_n,
                dtype: dtype.clone(),
                no_db: *no_db,
            };

            if let Err(err) = run_chat(chat_args) {
                eprintln!("Error running chat: {}", err);
            }
        }
    }
}
