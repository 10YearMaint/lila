use colored::Colorize;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct MarkdownMeta {
    pub output_filename: String,
}

pub fn extract_code_from_markdown(
    file_path: &str,
) -> io::Result<Result<HashMap<String, String>, String>> {
    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut meta_data = String::new();
    let mut in_front_matter = false;
    let mut found_meta = false;
    let mut code_blocks: HashMap<String, String> = HashMap::new();
    let mut current_lang = String::new();

    for line in reader.lines() {
        let line = line?;

        if line.trim() == "---" && !in_front_matter {
            in_front_matter = true;
        } else if line.trim() == "---" && in_front_matter {
            in_front_matter = false;
            found_meta = true;
        } else if in_front_matter {
            meta_data.push_str(&line);
            meta_data.push('\n');
        } else if line.trim().starts_with("```") && !current_lang.is_empty() {
            current_lang.clear();
        } else if line.trim().starts_with("```") {
            if line.contains(".python") {
                current_lang = "python".to_string();
            } else if line.contains(".rust") {
                current_lang = "rust".to_string();
            } else if line.contains("cpp") {
                current_lang = "cpp".to_string();
            } else if line.contains(".h") {
                current_lang = "h".to_string();
            }

            if !code_blocks.contains_key(&current_lang) {
                code_blocks.insert(current_lang.clone(), String::new());
            }
        } else if !current_lang.is_empty() {
            if let Some(code) = code_blocks.get_mut(&current_lang) {
                code.push_str(&line);
                code.push('\n');
            }
        }
    }

    if !found_meta {
        return Ok(Err("No metadata found".to_string()));
    }

    println!("Extracted YAML metadata:\n{}", meta_data);

    let cleaned_meta_data = meta_data.trim_end_matches("---").trim();
    let meta: MarkdownMeta = serde_yaml::from_str(cleaned_meta_data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("YAML parsing error: {}", e),
        )
    })?;

    let mut result: HashMap<String, String> = HashMap::new();
    for (lang, code) in code_blocks {
        let extension = match lang.as_str() {
            "python" => "py",
            "rust" => "rs",
            "cpp" => "cpp",
            "h" => "h",
            _ => continue,
        };

        let mut output_filename = meta.output_filename.clone();
        output_filename.push_str(&format!(".{}", extension));
        result.insert(output_filename, code);
    }

    Ok(Ok(result))
}

pub fn extract_code_from_folder(folder_path: &str, app_folder: &str) -> io::Result<()> {
    for entry in std::fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let sub_app_folder = PathBuf::from(app_folder).join(path.file_name().unwrap());
            std::fs::create_dir_all(&sub_app_folder)?;
            extract_code_from_folder(path.to_str().unwrap(), sub_app_folder.to_str().unwrap())?;
        } else if path.is_file() {
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                match extract_code_from_markdown(path.to_str().unwrap()) {
                    Ok(Ok(extracted_code)) => {
                        for (filename, code) in extracted_code {
                            let file_output_path = PathBuf::from(app_folder).join(filename);
                            if let Some(parent) = file_output_path.parent() {
                                std::fs::create_dir_all(parent)?;
                            }
                            let mut output_file = File::create(&file_output_path)?;
                            output_file.write_all(code.as_bytes())?;
                            let checkmark = "✔".green();
                            println!(
                                "{} Code extracted to {}",
                                checkmark,
                                file_output_path.display()
                            );
                        }
                    }
                    Ok(Err(_)) => {
                        // Copy simple markdown file to .app folder
                        let output_path = PathBuf::from(app_folder).join(path.file_name().unwrap());
                        std::fs::copy(&path, &output_path)?;
                        println!(
                            "{} {}",
                            "ℹ Copied file to".bright_cyan(),
                            output_path.display()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "{} {}: {}",
                            "! Error processing file".red(),
                            path.display(),
                            e
                        );
                    }
                }
            } else {
                // Copy non-markdown file to app folder
                let output_path = PathBuf::from(app_folder).join(path.file_name().unwrap());
                std::fs::copy(&path, &output_path)?;
                println!(
                    "{} {}",
                    "ℹ Copied file to ".bright_cyan(),
                    output_path.display()
                );
            }
        }
    }

    Ok(())
}
