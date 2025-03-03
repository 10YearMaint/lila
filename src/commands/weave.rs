use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Simple struct for YAML front matter.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarkdownMeta {
    pub output_filename: String,
    #[serde(default)]
    pub brief: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
}

/// Recursively copies all contents from `src` into `dst`.
pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
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

/// Infer the language to use in the fenced code block from a file extension.
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

/// Attempt to parse the front matter of a Markdown file,
/// returning Some(MarkdownMeta) if successful, else None.
///
/// We assume front matter is delimited by:
///
/// ```markdown
/// ---
/// # YAML lines...
/// ---
/// ```
///
/// at the top of the file.
fn parse_markdown_front_matter(file_path: &Path) -> io::Result<Option<MarkdownMeta>> {
    let f = File::open(file_path)?;
    let mut reader = BufReader::new(f);

    let mut first_line = String::new();
    // Read the first line; if it's not "---", no front matter.
    if reader.read_line(&mut first_line)? == 0 {
        return Ok(None);
    }
    if !first_line.trim().eq("---") {
        return Ok(None);
    }

    // Accumulate lines until we find another "---".
    let mut yaml_lines = Vec::new();
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            // No closing "---"; no valid front matter.
            return Ok(None);
        }
        if line.trim().eq("---") {
            // Reached the end of front matter.
            break;
        }
        yaml_lines.push(line);
    }

    // Join those lines into a single YAML string.
    let yaml_string = yaml_lines.join("");

    // Try parsing as MarkdownMeta
    match serde_yaml::from_str::<MarkdownMeta>(&yaml_string) {
        Ok(meta) => Ok(Some(meta)),
        Err(_) => Ok(None),
    }
}

/// Convert a single code file into a corresponding Markdown file.
/// Returns Ok(Some((output_path, meta))) if a new .md was generated,
/// or Ok(None) if it was skipped (already a Markdown file).
///
/// 1. Builds YAML front matter using `MarkdownMeta`.
/// 2. Infers the code block language from the file extension.
/// 3. Inserts the entire file content into a fenced code block.
pub fn convert_file_to_markdown(
    input_file: &Path,
    output_folder: &Path,
) -> io::Result<Option<(PathBuf, MarkdownMeta)>> {
    let extension = input_file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    // If extension is Markdown, skip converting (we'll handle the copy in the folder function).
    if extension == "md" || extension == "markdown" {
        println!(
            "{} {}",
            "ℹ Skipping Markdown file for conversion:".bright_cyan(),
            input_file.display()
        );
        return Ok(None);
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

    // By default, we only fill `output_filename`.
    // `brief` and `details` remain None unless provided in an existing .md file.
    let meta = MarkdownMeta {
        output_filename: file_stem.to_string(),
        brief: None,
        details: None,
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

    // Return the newly generated path + metadata so we can build content.md later
    Ok(Some((md_output_path, meta)))
}

/// Internal function that:
/// - Recursively walks a folder of code files.
/// - Converts each non-Markdown code file into a new `.md`.
/// - Copies existing `.md` / `.markdown` files as-is.
/// - Tries to parse their front matter for `MarkdownMeta`.
/// - Returns a list of `(PathBuf, MarkdownMeta)` for all files that have front matter
///   (both newly generated + any existing .md with valid front matter).
fn convert_folder_to_markdown_internal(
    input_folder: &str,
    output_folder: &str,
) -> io::Result<Vec<(PathBuf, MarkdownMeta)>> {
    let output_folder_path = PathBuf::from(output_folder);
    fs::create_dir_all(&output_folder_path)?;

    let mut generated_files = Vec::new();

    for entry in fs::read_dir(input_folder)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively handle subfolders
            let sub_output = output_folder_path.join(path.file_name().unwrap());
            fs::create_dir_all(&sub_output)?;
            // Recurse
            let sub_results = convert_folder_to_markdown_internal(
                path.to_str().unwrap(),
                sub_output.to_str().unwrap(),
            )?;
            // Extend our local results
            generated_files.extend(sub_results);
        } else if path.is_file() {
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if extension == "md" || extension == "markdown" {
                // 1) Copy the file.
                let dest_path = output_folder_path.join(path.file_name().unwrap());
                fs::copy(&path, &dest_path)?;
                let checkmark = "✔".green();
                println!(
                    "{} Copied {} -> {}",
                    checkmark,
                    path.display(),
                    dest_path.display()
                );

                // 2) Try to parse front matter to see if it has an output_filename (plus brief/details).
                if let Some(meta) = parse_markdown_front_matter(&path)? {
                    // If it has valid front matter, record it
                    generated_files.push((dest_path, meta));
                }
            } else {
                // Otherwise, convert the file into Markdown
                if let Some((md_path, meta)) = convert_file_to_markdown(&path, &output_folder_path)?
                {
                    generated_files.push((md_path, meta));
                }
            }
        }
    }

    Ok(generated_files)
}

/// Public function that creates the output folder structure,
/// converts/copies files, and then creates a single `content.md`
/// listing all Markdown files that have front matter with
/// `output_filename`, plus optional `brief` and `details`.
pub fn convert_folder_to_markdown(
    input_folder: &str,
    output_folder: &str,
) -> io::Result<Vec<PathBuf>> {
    // 1) Recursively gather all MD files that have front matter
    //    plus newly generated MD files that we know about.
    let generated_files = convert_folder_to_markdown_internal(input_folder, output_folder)?;

    // 2) Group files by their top-level chapter (folder) for building `content.md`.
    let output_folder_path = PathBuf::from(output_folder);
    let mut chapters: HashMap<String, Vec<(PathBuf, MarkdownMeta)>> = HashMap::new();

    for (md_file_path, meta) in &generated_files {
        // Determine the relative path from the output folder
        let relative_path = md_file_path
            .strip_prefix(&output_folder_path)
            .unwrap_or(&md_file_path);

        // Get the first component (chapter)
        let chapter = relative_path
            .components()
            .next()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| "Uncategorized".to_string());

        chapters
            .entry(chapter)
            .or_default()
            .push((md_file_path.clone(), meta.clone()));
    }

    // Sort chapters for consistent ordering
    let mut sorted_chapters: Vec<_> = chapters.into_iter().collect();
    sorted_chapters.sort_by_key(|(chapter, _)| chapter.clone());

    // 3) Create a top-level 'content.md' with an overview
    let book_content_md_path = output_folder_path.join("content.md");
    let mut book_content_md = File::create(&book_content_md_path)?;

    // Write the header
    writeln!(book_content_md, "# Book Overview")?;
    writeln!(book_content_md)?;
    writeln!(
        book_content_md,
        "Below is a list of all Markdown files that define an `output_filename` in \
        their front matter (if present). They are organized by chapters (folder names). \
        If a file also has a `brief` or `details`, you'll see them in the table.\n"
    )?;

    // Iterate over each chapter and write its table
    for (chapter_name, files) in sorted_chapters {
        writeln!(book_content_md, "## Chapter: {}\n", chapter_name)?;
        writeln!(
            book_content_md,
            "| **File Name** | **Path** | **Brief** | **Details** |"
        )?;
        writeln!(
            book_content_md,
            "|---------------|----------|-----------|-------------|"
        )?;

        for (md_file_path, meta) in files {
            let relative_path = md_file_path
                .strip_prefix(&output_folder_path)
                .unwrap_or(&md_file_path)
                .to_string_lossy();

            let brief = match &meta.brief {
                Some(text) => format!("✅ {}", text),
                None => "❌".to_string(),
            };
            let details = match &meta.details {
                Some(text) => format!("<details><summary>View Details</summary>{}</details>", text),
                None => "❌".to_string(),
            };

            writeln!(
                book_content_md,
                "| {} | [{}]({}) | {} | {} |",
                meta.output_filename, relative_path, relative_path, brief, details
            )?;
        }

        writeln!(book_content_md)?; // extra line
    }

    println!(
        "{} Created overview file at {}",
        "✔".green(),
        book_content_md_path.display()
    );

    // 4) Prepare the list of final .md files to return,
    //    i.e. everything from generated_files plus `content.md`.
    let mut all_md_paths: Vec<PathBuf> = generated_files
        .into_iter()
        .map(|(path, _meta)| path)
        .collect();

    all_md_paths.push(book_content_md_path);

    Ok(all_md_paths)
}
