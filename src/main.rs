use clap::Parser;
use colored::Colorize;
use dirs::home_dir;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

mod commands;
mod schema;
mod utils;

use commands::chat::ChatArgs;
use commands::edit::{edit_format_code_in_folder, edit_format_code_in_markdown};
use commands::render::translate_markdown_folder;
use commands::tangle::{extract_code_from_folder, extract_code_from_markdown};
use commands::weave::{convert_file_to_markdown, convert_folder_to_markdown};
use commands::{Args, Commands};
use utils::{env::ensure_pandoc_installed, utils::process_protocol_aimm};

fn main() {
    // Parse CLI args and load .env
    let args = Args::parse();
    dotenvy::dotenv().ok();

    let default_root = get_default_root();

    // Dispatch the command using dedicated helper functions.
    match args.command {
        Commands::Init => handle_init(),
        Commands::Tangle {
            file,
            folder,
            output,
            protocol,
        } => handle_tangle(file, folder, output, protocol, &default_root),
        Commands::Weave {
            file,
            folder,
            output,
        } => handle_weave(file, folder, output, &default_root),
        Commands::Edit { file, folder } => handle_edit(file, folder),
        Commands::Render {
            folder,
            output,
            css,
            mermaid,
            disable_mermaid,
        } => handle_render(folder, output, css, mermaid, disable_mermaid, &default_root),
        Commands::Save { db } => handle_save(db, &default_root),
        Commands::Rm { all, output } => handle_rm(all, output, &default_root),
        Commands::Chat {
            prompt,
            model_id,
            no_db,
        } => handle_chat(prompt, model_id, no_db),
    }
}

/// Returns the default project root as `<HOME>/.lila/<current_directory>`.
fn get_default_root() -> PathBuf {
    let home = home_dir().expect("Could not determine the home directory");
    let lila_root = home.join(".lila");
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let project_name = current_dir
        .file_name()
        .unwrap_or_else(|| OsStr::new("default"))
        .to_string_lossy()
        .to_string();
    lila_root.join(&project_name)
}

/// Initializes the lila environment.
fn handle_init() {
    if let Err(e) = commands::init::init() {
        eprintln!("Error during init: {}", e);
    }
}

/// Extracts code from a Markdown file or folder.
fn handle_tangle(
    file: Option<String>,
    folder: Option<String>,
    output: Option<String>,
    protocol: Option<String>,
    default_root: &Path,
) {
    let root_folder = output
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| match env::var("LILA_OUTPUT_PATH") {
            Ok(path) => Some(PathBuf::from(path)),
            Err(_) => Some(default_root.to_path_buf()),
        })
        .unwrap_or(default_root.to_path_buf());

    let app_folder = root_folder.join(".app");
    fs::create_dir_all(&app_folder)
        .unwrap_or_else(|e| panic!("Could not create .app folder: {}", e));

    if let Some(file) = file {
        match extract_code_from_markdown(&file) {
            Ok(Ok(extracted_code)) => {
                for (filename, code) in extracted_code {
                    let output_path = app_folder.join(filename);
                    if let Some(parent) = output_path.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    let mut output_file = File::create(&output_path).unwrap();
                    output_file.write_all(code.as_bytes()).unwrap();
                    println!(
                        "{} Code extracted to {}",
                        "✔".green(),
                        output_path.display()
                    );
                }
            }
            Ok(Err(_)) => {
                let output_path = app_folder.join(Path::new(&file).file_name().unwrap());
                fs::copy(&file, &output_path).unwrap();
                println!("Copied file to {}", output_path.display());
            }
            Err(e) => eprintln!("Error extracting code: {}", e),
        }
    } else if let Some(folder) = folder {
        if let Err(e) = extract_code_from_folder(&folder, &app_folder.to_string_lossy()) {
            eprintln!("Error extracting code from folder {}: {}", folder, e);
        }
    }

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

/// Converts source code back into Markdown.
fn handle_weave(
    file: Option<String>,
    folder: Option<String>,
    output: Option<String>,
    default_root: &Path,
) {
    let root_folder = output
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| match env::var("LILA_OUTPUT_PATH") {
            Ok(path) => Some(PathBuf::from(path).join("doc")),
            Err(_) => Some(default_root.join("doc")),
        })
        .unwrap_or_else(|| default_root.join("doc"));

    fs::create_dir_all(&root_folder)
        .unwrap_or_else(|e| panic!("Could not create output folder: {}", e));

    // We'll accumulate *all* created/converted .md files here.
    let mut all_markdown_paths = Vec::new();

    if let Some(file) = file {
        // Weave a single file into .md
        let input_path = PathBuf::from(&file);
        match convert_file_to_markdown(&input_path, &root_folder) {
            Ok(Some((md_out_path, _meta))) => {
                // If a .md was actually generated
                all_markdown_paths.push(md_out_path);
            }
            Ok(None) => {
                println!(
                    "Skipping file {} (already .md or something).",
                    input_path.display()
                );
            }
            Err(e) => eprintln!("Error converting file {}: {}", input_path.display(), e),
        }
    } else if let Some(folder) = folder {
        // Weave all code files in a folder into .md
        match convert_folder_to_markdown(&folder, &root_folder.to_string_lossy()) {
            Ok(md_paths) => {
                // We'll get back a list of .md files that were generated/copied
                all_markdown_paths = md_paths;
            }
            Err(e) => eprintln!("Error converting folder {}: {}", folder, e),
        }
    } else {
        eprintln!("No file or folder provided for conversion.");
        return;
    }

    // If we ended up with no .md files, just return
    if all_markdown_paths.is_empty() {
        println!("No Markdown files were generated or copied. Nothing to record.");
        return;
    }

    // Otherwise, write them to `created_markdown_files.txt` so the "save" command can pick them up.
    let created_files_list_path = root_folder.join("created_markdown_files.txt");
    let mut f = File::create(&created_files_list_path)
        .expect("Could not create created_markdown_files.txt");
    for path in &all_markdown_paths {
        writeln!(f, "{}", path.to_string_lossy())
            .expect("Could not write to created_markdown_files.txt");
    }

    println!(
        "{} Wrote list of .md files to {}",
        "✔".green(),
        created_files_list_path.display()
    );
}

/// Auto-formats code blocks in a Markdown file or folder.
fn handle_edit(file: Option<String>, folder: Option<String>) {
    if let Some(file) = file {
        if let Err(e) = edit_format_code_in_markdown(&file) {
            eprintln!("Error auto-formatting file {}: {}", file, e);
        }
    } else if let Some(folder) = folder {
        if let Err(e) = edit_format_code_in_folder(&folder) {
            eprintln!("Error auto-formatting folder {}: {}", folder, e);
        }
    } else {
        eprintln!("No file or folder provided for auto-formatting.");
    }
}

/// Translates Markdown into HTML.
fn handle_render(
    folder: String,
    output: Option<String>,
    css: Option<String>,
    mermaid: Option<String>,
    disable_mermaid: bool,
    default_root: &Path,
) {
    let root_folder = output
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| match env::var("LILA_OUTPUT_PATH") {
            Ok(path) => Some(PathBuf::from(path).join("doc")),
            Err(_) => Some(default_root.join("doc")),
        })
        .unwrap_or(default_root.join("doc"));

    fs::create_dir_all(&root_folder)
        .unwrap_or_else(|e| panic!("Could not create output folder: {}", e));

    if !ensure_pandoc_installed() {
        eprintln!("Pandoc is not installed. Please install Pandoc to use this tool.");
        std::process::exit(1);
    }

    let css_path = css.unwrap_or_else(|| "src/css/style.css".to_string());
    let mermaid_path = if disable_mermaid {
        None
    } else {
        Some(mermaid.unwrap_or_else(|| "src/js/mermaid.min.js".to_string()))
    };

    if let Err(e) = translate_markdown_folder(
        &folder,
        &root_folder.to_string_lossy(),
        &css_path,
        mermaid_path.as_ref().map(|s| s.as_str()),
    ) {
        eprintln!("Error translating markdown: {}", e);
    }
}

fn handle_save(db: Option<String>, default_root: &Path) {
    let db_folder = default_root.join("db");
    let db_path = db
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| db_folder.join("lila.db"));
    std::fs::create_dir_all(db_folder).expect("Could not create DB folder");

    let mut conn = commands::save::establish_connection(&db_path.to_string_lossy());

    let doc_folder = default_root.join("doc");
    let file_path = doc_folder.join("created_markdown_files.txt");

    if !file_path.exists() {
        eprintln!(
            "Error: '{}' does not exist. Did you run the 'weave' step yet?",
            file_path.display()
        );
        std::process::exit(1);
    }

    let created_files =
        std::fs::read_to_string(&file_path).expect("Unable to read created_markdown_files.txt");
    let files_to_save: Vec<String> = created_files.lines().map(|s| s.to_owned()).collect();

    if let Err(e) =
        commands::save::save_files_to_db(&files_to_save, &mut conn, &db_path.to_string_lossy())
    {
        eprintln!("Error saving Markdown files to DB: {e}");
    }

    println!(
        "Successfully saved md files to {}",
        db_path.display()
    );
}

/// Removes generated project files.
fn handle_rm(all: bool, output: Option<String>, default_root: &Path) {
    let root_folder = output
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_root.to_path_buf());
    if let Err(e) = commands::remove::remove_output_folder(&root_folder.to_string_lossy(), all) {
        eprintln!("Error removing project files: {}", e);
    }
}

/// Constructs a ChatArgs struct and runs the chat subcommand.
fn handle_chat(prompt: Option<String>, model_id: Option<String>, no_db: bool) {
    let chat_args = ChatArgs {
        prompt,
        model_id,
        no_db,
    };
    if let Err(e) = commands::chat::run_chat(chat_args) {
        eprintln!("Error running chat: {}", e);
    }
}
