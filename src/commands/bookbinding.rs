use colored::Colorize;
use regex::Regex;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Recursively copies all contents from `src` into `dst`.
fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Inline placeholders in a Markdown file.
fn inline_placeholders_in_file(file_path: &Path) -> io::Result<()> {
    let content = fs::read_to_string(file_path)?;
    let parent = file_path.parent().unwrap_or_else(|| Path::new(""));

    let re = Regex::new(r"@\{([^}]+)\}").unwrap();

    let new_content = re.replace_all(&content, |caps: &regex::Captures| {
        let referenced = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if let Some((file_name, identifier)) = referenced.split_once(':') {
            let ref_path = parent.join(file_name);
            if ref_path.exists() {
                if let Ok(Some(def)) = extract_definition_from_file(&ref_path, identifier) {
                    let ext = Path::new(file_name)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if let Some(lang) = infer_language_from_extension(&ext) {
                        return format!("\n\n```{{.{} .cb-code}}\n{}\n```", lang, def);
                    } else {
                        return format!("\n\n```\n{}\n```", def);
                    }
                }
            }
            // If file not found or extraction fails, leave the placeholder unchanged.
            return caps.get(0).unwrap().as_str().to_string();
        } else {
            // No identifier provided; include the entire file.
            let ref_path = parent.join(referenced);
            if ref_path.exists() {
                if let Ok(file_content) = fs::read_to_string(&ref_path) {
                    let ext = Path::new(referenced)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if let Some(lang) = infer_language_from_extension(&ext) {
                        return format!("\n\n```{{.{} .cb-code}}\n{}\n```", lang, file_content);
                    } else {
                        return file_content;
                    }
                }
            }
            return caps.get(0).unwrap().as_str().to_string();
        }
    });

    fs::write(file_path, new_content.as_ref())?;
    Ok(())
}

/// Recursively inlines placeholders in all Markdown files in the given folder.
pub fn inline_placeholders_in_readmes_in_folder(folder: &Path) -> io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            inline_placeholders_in_readmes_in_folder(&path)?;
        } else if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("md") {
                    inline_placeholders_in_file(&path)?;
                }
            }
        }
    }
    Ok(())
}

/// Recursively copies only Markdown files from the source folder to the destination folder,
/// preserving the directory structure.
pub fn copy_markdown_files(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let sub_dst = dst.join(entry.file_name());
            copy_markdown_files(&path, &sub_dst)?;
        } else if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("md") {
                    let dest_file = dst.join(entry.file_name());
                    fs::copy(&path, &dest_file)?;
                    println!(
                        "{} Copied {} -> {}",
                        "✔".green(),
                        path.display(),
                        dest_file.display()
                    );
                }
            }
        }
    }
    Ok(())
}

/// Processes book binding by first copying the input folder to a temporary folder,
/// inlining placeholders in the temporary folder, and then copying only Markdown files
/// to the final output folder. The original input folder remains untouched.
pub fn process_bookbinding(input_folder: &str, output_folder: &str) -> io::Result<()> {
    let input_path = Path::new(input_folder);
    let output_path = Path::new(output_folder);

    // Create a temporary folder inside the output folder.
    let temp_folder = output_path.join("temp_inlined_source");
    let _ = fs::remove_dir_all(&temp_folder); // Remove any existing temporary folder.
    fs::create_dir_all(&temp_folder)?;

    // Copy the entire input folder to the temporary folder.
    copy_dir_all(input_path, &temp_folder)?;

    // Inline placeholders in all Markdown files within the temporary folder.
    inline_placeholders_in_readmes_in_folder(&temp_folder)?;

    // Copy only Markdown files from the temporary folder to the final output folder.
    copy_markdown_files(&temp_folder, output_path)?;

    // Remove the temporary folder.
    fs::remove_dir_all(&temp_folder)?;

    println!(
        "{} Book binding complete. Markdown files copied to {}.",
        "✔".green(),
        output_path.display()
    );
    Ok(())
}

/// Extracts a definition (function or class) from a source file by identifier.
/// Supports basic heuristics for Python and Rust.
fn extract_definition_from_file(file_path: &Path, identifier: &str) -> io::Result<Option<String>> {
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut result_lines = Vec::new();
    let mut in_def = false;
    let mut header_indent: Option<usize> = None;

    for line in reader.lines() {
        let line = line?;
        if !in_def {
            let trimmed = line.trim_start();
            if ext == "py" {
                if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
                    if let Some(rest) = trimmed.strip_prefix("def ") {
                        if let Some(idx) = rest.find('(') {
                            let name = rest[..idx].trim();
                            if name == identifier {
                                in_def = true;
                                header_indent =
                                    Some(line.chars().take_while(|c| c.is_whitespace()).count());
                                result_lines.push(line);
                            }
                        }
                    } else if let Some(rest) = trimmed.strip_prefix("class ") {
                        let name = rest
                            .split(|c| c == ':' || c == '(')
                            .next()
                            .unwrap_or("")
                            .trim();
                        if name == identifier {
                            in_def = true;
                            header_indent =
                                Some(line.chars().take_while(|c| c.is_whitespace()).count());
                            result_lines.push(line);
                        }
                    }
                }
            } else if ext == "rs" {
                if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
                    let without_pub = if trimmed.starts_with("pub fn ") {
                        &trimmed[7..]
                    } else {
                        &trimmed[3..]
                    };
                    if without_pub.starts_with(identifier) {
                        let post = without_pub.chars().nth(identifier.len());
                        if post == Some('(') || post == Some(' ') {
                            in_def = true;
                            header_indent =
                                Some(line.chars().take_while(|c| c.is_whitespace()).count());
                            result_lines.push(line);
                        }
                    }
                }
            }
        } else {
            if ext == "py" {
                let current_indent = line.chars().take_while(|c| c.is_whitespace()).count();
                if line.trim().is_empty() || current_indent > header_indent.unwrap_or(0) {
                    result_lines.push(line);
                } else {
                    break;
                }
            } else if ext == "rs" {
                result_lines.push(line.clone());
                let joined: String = result_lines.join("\n");
                let open_braces = joined.matches('{').count();
                let close_braces = joined.matches('}').count();
                if open_braces > 0 && open_braces == close_braces {
                    break;
                }
            }
        }
    }

    if result_lines.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result_lines.join("\n")))
    }
}

/// Infers the language for a fenced code block based on file extension.
fn infer_language_from_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "py" => Some("python"),
        "rs" => Some("rust"),
        "cpp" => Some("cpp"),
        "c" => Some("c"),
        "h" => Some("c"),
        "js" => Some("javascript"),
        "ts" => Some("typescript"),
        "sh" => Some("bash"),
        _ => None,
    }
}
