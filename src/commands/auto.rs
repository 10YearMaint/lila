use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// A simple enum to track recognized languages.
#[derive(Debug, PartialEq)]
enum CodeLanguage {
    Python,
    Rust,
    Unknown,
}

/// Detect the language from a Markdown fence line (e.g. ```{.python}).
fn detect_language_from_line(line: &str) -> CodeLanguage {
    let lower_line = line.to_lowercase();

    if lower_line.contains(".python") || lower_line.contains("python") || lower_line.contains(".py")
    {
        CodeLanguage::Python
    } else if lower_line.contains(".rust")
        || lower_line.contains("rust")
        || lower_line.contains(".rs")
    {
        CodeLanguage::Rust
    } else {
        CodeLanguage::Unknown
    }
}

/// Format the snippet in `code_lines` using the relevant formatter based on `lang`.
fn format_code_snippet(code_lines: &[String], lang: &CodeLanguage) -> io::Result<Vec<String>> {
    if *lang == CodeLanguage::Unknown {
        // If unknown, do nothing and return lines unchanged.
        return Ok(code_lines.to_vec());
    }

    // Decide extension and formatter.
    let (extension, formatter, formatter_args) = match lang {
        CodeLanguage::Python => ("py", "black", vec!["--quiet"]),
        CodeLanguage::Rust => ("rs", "rustfmt", vec![]),
        CodeLanguage::Unknown => unreachable!("We handled Unknown already"),
    };

    // Create a temp file and rename with correct extension.
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().with_extension(extension);
    fs::rename(temp_file.path(), &temp_path)?;

    // Write the code block to the temporary file.
    {
        let mut f = File::create(&temp_path)?;
        for line in code_lines {
            writeln!(f, "{}", line)?;
        }
        f.flush()?;
    }

    // Call the formatter silently.
    let status = Command::new(formatter)
        .args(&formatter_args)
        .arg(&temp_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => {
            // Read back the newly formatted code.
            let formatted_code = fs::read_to_string(&temp_path)?;
            let formatted_lines = formatted_code
                .lines()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            Ok(formatted_lines)
        }
        Ok(_) => {
            eprintln!(
                "Warning: formatter exited with a non-zero status for {:?}",
                lang
            );
            Ok(code_lines.to_vec()) // Return the original snippet on failure
        }
        Err(e) => {
            eprintln!("Error running formatter for {:?}: {}", lang, e);
            Ok(code_lines.to_vec()) // Return the original snippet on error
        }
    }
}

/// Auto-format code blocks (Python, Rust, etc.) in a single Markdown file in-place.
pub fn auto_format_code_in_markdown(file_path: &str) -> io::Result<()> {
    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut lines: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_language = CodeLanguage::Unknown;
    let mut code_block_lines: Vec<String> = Vec::new();
    let mut code_block_start_index = 0;

    for line_result in reader.lines() {
        let line = line_result?;

        if line.trim().starts_with("```") {
            // Check if we're closing an existing code block or opening a new one
            if in_code_block {
                // Closing fence
                if code_block_language != CodeLanguage::Unknown {
                    match format_code_snippet(&code_block_lines, &code_block_language) {
                        Ok(formatted_lines) => {
                            let block_len = code_block_lines.len();
                            lines.drain(code_block_start_index..code_block_start_index + block_len);
                            for (i, fl) in formatted_lines.iter().enumerate() {
                                lines.insert(code_block_start_index + i, fl.to_string());
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: could not format {:?} code block in {}: {}",
                                code_block_language, file_path, e
                            );
                        }
                    }
                    code_block_lines.clear();
                }

                in_code_block = false;
                code_block_language = CodeLanguage::Unknown;
            } else {
                // Opening fence
                in_code_block = true;
                code_block_start_index = lines.len() + 1; // +1 because we haven't pushed the fence line yet
                code_block_language = detect_language_from_line(&line);
            }

            lines.push(line);
        } else if in_code_block {
            // Inside the code block
            code_block_lines.push(line.clone());
            lines.push(line);
        } else {
            // Outside any code block
            lines.push(line);
        }
    }

    // If file ends but code block wasn't closed, we won't format that trailing block.
    // Overwrite the original file with updated lines.
    let mut output = File::create(&path)?;
    for l in &lines {
        writeln!(output, "{}", l)?;
    }

    Ok(())
}

/// Recursively auto-format code blocks in all `.md` files under `folder_path`.
/// Similar logic to `extract_code_from_folder` in your extract command.
pub fn auto_format_code_in_folder(folder_path: &str) -> io::Result<()> {
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively handle subfolders
            auto_format_code_in_folder(path.to_str().unwrap())?;
        } else if path.is_file() {
            // Only auto-format if it's a Markdown file
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                println!("Auto-formatting {:?}", path.display());
                if let Err(e) = auto_format_code_in_markdown(path.to_str().unwrap()) {
                    eprintln!("Error formatting {}: {}", path.display(), e);
                }
            }
            // else: For non-markdown files, do nothing (or handle differently if desired).
        }
    }
    Ok(())
}
