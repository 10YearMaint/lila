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
        } => handle_chat(
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
        ),
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
        .unwrap_or(default_root.join("doc"));

    fs::create_dir_all(&root_folder)
        .unwrap_or_else(|e| panic!("Could not create output folder: {}", e));

    if let Some(file) = file {
        let input_path = PathBuf::from(&file);
        if let Err(e) = convert_file_to_markdown(&input_path, &root_folder) {
            eprintln!("Error converting file {}: {}", input_path.display(), e);
        }
    } else if let Some(folder) = folder {
        if let Err(e) = convert_folder_to_markdown(&folder, &root_folder.to_string_lossy()) {
            eprintln!("Error converting folder {}: {}", folder, e);
        }
    } else {
        eprintln!("No file or folder provided for conversion.");
    }
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

/// Saves rendered HTML metadata to a SQLite database.
fn handle_save(db: Option<String>, default_root: &Path) {
    let doc_pure_folder = default_root.join("doc_pure");
    let file_path = doc_pure_folder.join("created_html_files.txt");

    if !file_path.exists() {
        eprintln!(
            "Error: '{}' does not exist. Please run the 'translate' command first.",
            file_path.display()
        );
        std::process::exit(1);
    }

    let created_files =
        fs::read_to_string(&file_path).expect("Unable to read created_html_files.txt");
    let html_files: Vec<String> = created_files.lines().map(|s| s.to_string()).collect();

    let db_path = db
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| doc_pure_folder.join("lila.db"));

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("Could not create database directory: {}", e));
    }

    let mut conn = commands::save::establish_connection(&db_path.to_string_lossy());
    if let Err(e) =
        commands::save::save_html_metadata_to_db(&html_files, &mut conn, &db_path.to_string_lossy())
    {
        eprintln!("Error saving HTML metadata to database: {}", e);
    } else {
        println!(
            "Successfully saved HTML metadata to '{}'",
            db_path.display()
        );
    }
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
fn handle_chat(
    cpu: bool,
    tracing: bool,
    verbose_prompt: bool,
    prompt: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    seed: u64,
    sample_len: usize,
    model_id: Option<String>,
    model: String,
    revision: Option<String>,
    weight_file: Option<String>,
    tokenizer: Option<String>,
    quantized: bool,
    repeat_penalty: f32,
    repeat_last_n: usize,
    dtype: Option<String>,
    no_db: bool,
) {
    let chat_args = ChatArgs {
        cpu,
        tracing,
        verbose_prompt,
        prompt,
        temperature: Some(temperature.unwrap_or(0.0)),
        top_p: Some(top_p.unwrap_or(0.0)),
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
    };
    if let Err(e) = commands::chat::run_chat(chat_args) {
        eprintln!("Error running chat: {}", e);
    }
}
