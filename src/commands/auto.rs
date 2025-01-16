use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Auto-format Python code blocks in a given Markdown file using `black`,
/// then re-insert the formatted code into the Markdown in place.
pub fn auto_format_code_in_markdown(file_path: &str) -> io::Result<()> {
    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let mut temp_file = NamedTempFile::new()?;

    let mut lines: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut is_python_block = false;
    let mut code_block_lines: Vec<String> = Vec::new();
    let mut code_block_start_index: usize = 0; // Where in `lines` we started the code block

    // Read file lines into memory
    for line in reader.lines() {
        let line = line?;

        // Check if we hit a triple-backtick
        if line.trim().starts_with("```") {
            // If we were already in a code block, this is the closing fence
            if in_code_block {
                // If it was a Python block, format the collected lines
                if is_python_block {
                    // Format the python code block lines using `black`
                    // 1) Write to a temp file
                    let mut temp_file = tempfile::NamedTempFile::new()?;
                    for code_line in &code_block_lines {
                        writeln!(temp_file, "{}", code_line)?;
                    }
                    temp_file.flush()?;

                    // 2) Run `black <tempfile>`
                    let status = Command::new("black")
                        .arg("--quiet")
                        .arg(temp_file.path())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            // 3) Read back the formatted code
                            let formatted_code = fs::read_to_string(temp_file.path())?;

                            // Replace the lines in `lines` between code_block_start_index and current
                            let formatted_code_lines: Vec<&str> = formatted_code.lines().collect();
                            let block_len = code_block_lines.len();

                            // Clear out the original unformatted lines
                            lines.drain(code_block_start_index..(code_block_start_index + block_len));

                            // Insert the newly formatted lines
                            for (i, formatted_line) in formatted_code_lines.iter().enumerate() {
                                lines.insert(code_block_start_index + i, formatted_line.to_string());
                            }
                        }
                        Ok(_) => {
                            eprintln!(
                                "Warning: `black` exited with a non-zero status for {}",
                                file_path
                            );
                        }
                        Err(e) => {
                            eprintln!("Error running `black`: {}", e);
                        }
                    }

                    // Reset the temporary buffer
                    code_block_lines.clear();
                }

                // Close code block
                in_code_block = false;
                is_python_block = false;
            } else {
                // Opening fence
                in_code_block = true;
                code_block_start_index = lines.len() + 1; // +1 because this line hasn't been pushed yet
                is_python_block =
                    line.contains(".python") || line.contains("python") || line.contains(".py");
            }

            // In all cases, add the fence line to `lines`
            lines.push(line);
        } else if in_code_block {
            // We are inside a code block
            code_block_lines.push(line.clone());
            lines.push(line);
        } else {
            // Normal line outside code block
            lines.push(line);
        }
    }

    // Finally, overwrite the original file with the new lines
    let mut output = File::create(&path)?;
    for line in &lines {
        writeln!(output, "{}", line)?;
    }

    Ok(())
}
