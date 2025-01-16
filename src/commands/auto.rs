use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Auto-format code blocks (Python, Rust) in a given Markdown file.
/// It detects Python or Rust code blocks by looking for e.g.
/// \`\`\`{.python} or \`\`\`rust fences.
pub fn auto_format_code_in_markdown(file_path: &str) -> io::Result<()> {
    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut lines: Vec<String> = Vec::new();
    let mut in_code_block = false;

    // We'll store which language we detected for the currently active code block.
    let mut code_block_language = CodeLanguage::Unknown;

    // Temporary buffer for the lines inside the code block.
    let mut code_block_lines: Vec<String> = Vec::new();

    // We'll note where the code block started in `lines`, so we know where to re-insert after formatting.
    let mut code_block_start_index: usize = 0;

    for line_result in reader.lines() {
        let line = line_result?;

        // Check if this line is a fence (```...).
        if line.trim().starts_with("```") {
            if in_code_block {
                // This must be the closing fence.
                // Attempt formatting if the block is recognized (Python/Rust).
                if code_block_language != CodeLanguage::Unknown {
                    match format_code_snippet(&code_block_lines, &code_block_language) {
                        Ok(formatted_code_lines) => {
                            // Remove the old, unformatted code lines from `lines`.
                            let block_len = code_block_lines.len();
                            lines.drain(
                                code_block_start_index..(code_block_start_index + block_len),
                            );

                            // Insert newly formatted lines in place.
                            for (i, formatted_line) in formatted_code_lines.iter().enumerate() {
                                lines
                                    .insert(code_block_start_index + i, formatted_line.to_string());
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Could not format {:?} code block in {}:\n{}",
                                code_block_language, file_path, e
                            );
                        }
                    }
                    // Reset the snippet buffer after we finish formatting.
                    code_block_lines.clear();
                }

                // End the code block.
                in_code_block = false;
                code_block_language = CodeLanguage::Unknown;
            } else {
                // We are opening a new code block.
                in_code_block = true;
                code_block_start_index = lines.len() + 1; // +1 because we haven't yet pushed the fence line.

                // Detect language from the fence line.
                code_block_language = detect_language_from_line(&line);
            }

            // Either way (open or close), push the fence line itself to `lines`.
            lines.push(line);
        } else if in_code_block {
            // We are in the middle of a code block. Accumulate the lines for possible formatting.
            code_block_lines.push(line.clone());
            lines.push(line);
        } else {
            // Normal line (outside any code block).
            lines.push(line);
        }
    }

    // Overwrite the original file with the updated lines.
    let mut output = File::create(&path)?;
    for line in &lines {
        writeln!(output, "{}", line)?;
    }

    Ok(())
}

/// A simple enum to track recognized languages.
#[derive(Debug, PartialEq)]
enum CodeLanguage {
    Python,
    Rust,
    Unknown,
}

/// Checks the opening fence line for `.python`, `.rust`, etc.
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

/// Formats the snippet in `code_lines` based on `lang`, returning the newly formatted lines.
/// - Python => `black`
/// - Rust => `rustfmt`
/// If something goes wrong, it returns an error or simply logs a warning.
fn format_code_snippet(code_lines: &[String], lang: &CodeLanguage) -> io::Result<Vec<String>> {
    // If unknown, do nothing.
    if *lang == CodeLanguage::Unknown {
        return Ok(code_lines.to_vec());
    }

    // Decide which file extension we need.
    let extension = match lang {
        CodeLanguage::Python => "py",
        CodeLanguage::Rust => "rs",
        CodeLanguage::Unknown => unreachable!(), // we already handled Unknown above
    };

    // Create a temp file. We'll rename it to have the appropriate extension
    // so that the formatter recognizes it properly.
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().with_extension(extension);

    // The default `NamedTempFile` path has no extension, so we'll rename:
    fs::rename(temp_file.path(), &temp_path)?;

    // Write the code block lines to the temp file with extension.
    {
        let mut temp_file_with_ext = File::create(&temp_path)?;
        for code_line in code_lines {
            writeln!(temp_file_with_ext, "{}", code_line)?;
        }
        temp_file_with_ext.flush()?;
    }

    // Figure out which formatter and arguments to run.
    let (formatter, args) = match lang {
        CodeLanguage::Python => ("black", vec!["--quiet"]),
        CodeLanguage::Rust => ("rustfmt", vec![]),
        CodeLanguage::Unknown => unreachable!(),
    };

    // Run the formatter silently.
    let status = Command::new(formatter)
        .args(&args)
        .arg(&temp_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // If the formatter succeeded, read back the newly formatted code.
    match status {
        Ok(s) if s.success() => {
            let formatted_code = fs::read_to_string(&temp_path)?;
            let formatted_code_lines = formatted_code
                .lines()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            Ok(formatted_code_lines)
        }
        Ok(_) => {
            eprintln!(
                "Warning: formatter exited with a non-zero status for {:?}",
                lang
            );
            // Return the original code lines unmodified if there's a formatting error.
            Ok(code_lines.to_vec())
        }
        Err(e) => {
            eprintln!("Error running formatter for {:?}: {}", lang, e);
            // Return the original snippet on error.
            Ok(code_lines.to_vec())
        }
    }
}
