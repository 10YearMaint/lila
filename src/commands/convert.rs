use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Simple struct for YAML front matter. You can expand this as needed.
#[derive(Debug, Serialize)]
pub struct MarkdownMeta {
    pub output_filename: String,
}

/// Convert a single code file into a corresponding Markdown file:
/// 1. Builds YAML front matter using `MarkdownMeta`.
/// 2. Infers the code block language from the file extension.
/// 3. Inserts the entire file content into a fenced code block.
pub fn convert_file_to_markdown(input_file: &Path, output_folder: &Path) -> io::Result<()> {
    // Infer the extension => language
    let extension = input_file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Skip `.md` or `.markdown` files
    if extension == "md" || extension == "markdown" {
        println!("ℹ Skipping Markdown file: {}", input_file.display());
        return Ok(());
    }

    // Match file extensions to language labels for code blocks
    let lang = match extension.as_str() {
        "py" => "python",
        "rs" => "rust",
        "cpp" => "cpp",
        "c" => "c",
        "h" => "c",
        "js" => "javascript",
        "ts" => "typescript",
        "sh" => "bash",
        _ => "",
    };

    // Use the file stem (e.g. `main` from `main.rs`) for the front matter
    let file_stem = input_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let meta = MarkdownMeta {
        output_filename: file_stem.to_string(),
    };

    // Serialize the metadata into YAML
    let yaml = serde_yaml::to_string(&meta).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("YAML serialization error: {}", e),
        )
    })?;

    // Construct the final .md filename: e.g. `main.rs` -> `main.md`
    let md_filename = format!("{}.md", file_stem);
    let md_output_path = output_folder.join(md_filename);

    // Read the code from the input file
    let file = File::open(input_file)?;
    let reader = BufReader::new(file);
    let mut code_content = String::new();
    for line in reader.lines() {
        code_content.push_str(&line?);
        code_content.push('\n');
    }

    // Write out the combined Markdown:
    // 1) front matter delimited by `---`
    // 2) a blank line
    // 3) fenced code block
    let mut md_file = File::create(&md_output_path)?;
    writeln!(md_file, "---")?;
    write!(md_file, "{}", yaml)?; // serde_yaml output includes a trailing newline
    writeln!(md_file, "---")?;
    writeln!(md_file)?; // one blank line after the front matter

    // Begin code block
    if lang.is_empty() {
        writeln!(md_file, "```")?;
    } else {
        writeln!(md_file, "```{}", lang)?;
    }
    write!(md_file, "{}", code_content)?;
    writeln!(md_file, "```")?;

    // Print success line with a checkmark
    println!(
        "✔ Converted {} -> {}",
        input_file.display(),
        md_output_path.display()
    );

    Ok(())
}

/// Recursively walk a folder of code files, converting each to Markdown,
/// and skip `.md` or `.markdown` files.
pub fn convert_folder_to_markdown(input_folder: &str, output_folder: &str) -> io::Result<()> {
    let output_folder_path = PathBuf::from(output_folder);
    fs::create_dir_all(&output_folder_path)?;

    for entry in fs::read_dir(input_folder)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recreate the same structure in the output
            let sub_output = output_folder_path.join(path.file_name().unwrap());
            fs::create_dir_all(&sub_output)?;
            convert_folder_to_markdown(path.to_str().unwrap(), sub_output.to_str().unwrap())?;
        } else if path.is_file() {
            // Skip Markdown files
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") {
                    println!("ℹ Skipping Markdown file: {}", path.display());
                    continue;
                }
            }
            convert_file_to_markdown(&path, &output_folder_path)?;
        }
    }

    Ok(())
}
