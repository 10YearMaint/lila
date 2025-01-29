use colored::Colorize;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Simple struct for YAML front matter. You can expand this as needed.
#[derive(Debug, Serialize)]
pub struct MarkdownMeta {
    pub output_filename: String,
}

/// Convert a single code file into a corresponding Markdown file.
/// 1. Builds YAML front matter using `MarkdownMeta`.
/// 2. Infers the code block language from the file extension.
/// 3. Inserts the entire file content into a fenced code block.
pub fn convert_file_to_markdown(input_file: &Path, output_folder: &Path) -> io::Result<()> {
    let extension = input_file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Skip `.md` or `.markdown`
    if extension == "md" || extension == "markdown" {
        println!(
            "{} {}",
            "ℹ Skipping Markdown file:".bright_cyan(),
            input_file.display()
        );
        return Ok(());
    }

    // Determine code block language
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

    let file_stem = input_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let meta = MarkdownMeta {
        output_filename: file_stem.to_string(),
    };

    let yaml = serde_yaml::to_string(&meta).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("YAML serialization error: {}", e),
        )
    })?;

    // Construct output path, e.g. `main.md`
    let md_filename = format!("{}.md", file_stem);
    let md_output_path = output_folder.join(md_filename);

    // Read code file contents
    let file = File::open(input_file)?;
    let reader = BufReader::new(file);
    let mut code_content = String::new();
    for line in reader.lines() {
        code_content.push_str(&line?);
        code_content.push('\n');
    }

    // Write out our combined Markdown
    {
        let mut md_file = File::create(&md_output_path)?;
        writeln!(md_file, "---")?;
        write!(md_file, "{}", yaml)?;
        writeln!(md_file, "---")?;
        writeln!(md_file)?;

        if lang.is_empty() {
            writeln!(md_file, "```")?;
        } else {
            writeln!(md_file, "```{}", lang)?;
        }

        write!(md_file, "{}", code_content)?;
        writeln!(md_file, "```")?;
    }

    let checkmark = "✔".green();
    println!(
        "{} Converted {} -> {}",
        checkmark,
        input_file.display(),
        md_output_path.display()
    );

    Ok(())
}

/// Recursively walk a folder of code files, converting each to Markdown.
/// Skips `.md` or `.markdown` files.
pub fn convert_folder_to_markdown(input_folder: &str, output_folder: &str) -> io::Result<()> {
    let output_folder_path = PathBuf::from(output_folder);
    fs::create_dir_all(&output_folder_path)?;

    for entry in fs::read_dir(input_folder)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let sub_output = output_folder_path.join(path.file_name().unwrap());
            fs::create_dir_all(&sub_output)?;
            convert_folder_to_markdown(path.to_str().unwrap(), sub_output.to_str().unwrap())?;
        } else if path.is_file() {
            // Skip Markdown files
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") {
                    println!(
                        "{} {}",
                        "ℹ Skipping Markdown file:".bright_cyan(),
                        path.display()
                    );
                    continue;
                }
            }
            convert_file_to_markdown(&path, &output_folder_path)?;
        }
    }

    Ok(())
}
