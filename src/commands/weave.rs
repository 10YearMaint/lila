use colored::Colorize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
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

/// Recursively ensures that each folder in the given directory has a README.md file.
/// If a README.md exists, it updates it by appending file mentions (in the format "@{filename}")
/// for any files not already mentioned.
pub fn prepare_readme_in_folder(folder: &Path) -> io::Result<()> {
    if folder.is_dir() {
        let readme_path = folder.join("README.md");
        let mut existing_mentions = HashSet::new();
        let mut existing_content = String::new();

        if readme_path.exists() {
            existing_content = fs::read_to_string(&readme_path)?;
            for line in existing_content.lines() {
                if let Some(start) = line.find("@{") {
                    if let Some(end) = line[start..].find("}") {
                        let mention = &line[start + 2..start + end];
                        // If the mention contains a colon, split and use the file name before the colon.
                        let file_mention =
                            mention.split_once(':').map_or(mention, |(file, _)| file);
                        existing_mentions.insert(file_mention.to_string());
                    }
                }
            }
        } else {
            fs::write(&readme_path, "")?;
        }

        let mut new_mentions = Vec::new();
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                    if fname.eq_ignore_ascii_case("README.md") {
                        continue;
                    }
                    if !existing_mentions.contains(fname) {
                        new_mentions.push(fname.to_string());
                    }
                }
            }
        }

        if !new_mentions.is_empty() {
            let mut file = OpenOptions::new().append(true).open(&readme_path)?;
            for mention in new_mentions {
                writeln!(file, "@{{{}}}", mention)?;
            }
            println!("Updated README.md at {}", readme_path.display());
        }
    }

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            prepare_readme_in_folder(&path)?;
        }
    }
    Ok(())
}

/// Extracts the definition (function or class) identified by `identifier` from the given file.
/// Supports basic heuristics for Python and Rust files.
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
                // Look for Python definitions
                if (trimmed.starts_with("def ") || trimmed.starts_with("class ")) {
                    // tokens[1] should contain the name plus potential parameters.
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
                        // For class definitions, check for ':' or '('
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
                // Look for Rust definitions: "fn identifier(" or "pub fn identifier("
                if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
                    let without_pub = if trimmed.starts_with("pub fn ") {
                        &trimmed[7..]
                    } else {
                        &trimmed[3..]
                    };
                    if without_pub.starts_with(identifier) {
                        // Ensure that the next character is '(' or a space.
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
            // We are inside a definition block.
            if ext == "py" {
                // For Python, include lines that are blank or indented more than the header.
                let current_indent = line.chars().take_while(|c| c.is_whitespace()).count();
                if line.trim().is_empty() || current_indent > header_indent.unwrap_or(0) {
                    result_lines.push(line);
                } else {
                    break;
                }
            } else if ext == "rs" {
                // For Rust, use a simple brace counter.
                result_lines.push(line.clone());
                // Count braces in the collected lines.
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

/// Scans a given file (expected to be a README.md) for placeholders of the form "@{...}".
/// If the placeholder is of the form "@{filename:identifier}", only the corresponding
/// definition (function or class) from the file is inlined. Otherwise, the entire file content
/// is inlined.
fn inline_placeholders_in_file(file_path: &Path) -> io::Result<()> {
    let content = fs::read_to_string(file_path)?;
    let parent = file_path.parent().unwrap_or_else(|| Path::new(""));

    let re = Regex::new(r"@\{([^}]+)\}").unwrap();

    let new_content = re.replace_all(&content, |caps: &regex::Captures| {
        let referenced = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        // Check if the placeholder specifies an identifier.
        let replacement = if let Some((file_name, identifier)) = referenced.split_once(':') {
            let ref_path = parent.join(file_name);
            if ref_path.exists() {
                match extract_definition_from_file(&ref_path, identifier) {
                    Ok(Some(def)) => def,
                    _ => caps.get(0).unwrap().as_str().to_string(),
                }
            } else {
                caps.get(0).unwrap().as_str().to_string()
            }
        } else {
            // No identifier specified, inline the entire file.
            let ref_path = parent.join(referenced);
            if ref_path.exists() {
                match fs::read_to_string(&ref_path) {
                    Ok(file_content) => file_content,
                    Err(_) => caps.get(0).unwrap().as_str().to_string(),
                }
            } else {
                caps.get(0).unwrap().as_str().to_string()
            }
        };
        // Prepend two newlines to add an empty line before the inserted content.
        format!("\n\n{}", replacement)
    });

    fs::write(file_path, new_content.as_ref())?;
    println!("Inlined placeholders in {}", file_path.display());
    Ok(())
}

/// Recursively finds README.md files in the given folder and inlines their placeholders.
pub fn inline_placeholders_in_readmes_in_folder(folder: &Path) -> io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            inline_placeholders_in_readmes_in_folder(&path)?;
        } else if path.is_file() {
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if fname.eq_ignore_ascii_case("README.md") {
                    inline_placeholders_in_file(&path)?;
                }
            }
        }
    }
    Ok(())
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
                let dest_path = output_folder_path.join(path.file_name().unwrap());
                // Only copy if the source and destination are not the same.
                if fs::canonicalize(&path)? != fs::canonicalize(&dest_path)? {
                    fs::copy(&path, &dest_path)?;
                    println!(
                        "{} Copied {} -> {}",
                        "✔".green(),
                        path.display(),
                        dest_path.display()
                    );
                } else {
                    println!(
                        "Skipping copying Markdown file {} as it is already in place.",
                        path.display()
                    );
                }

                // Try to parse the front matter to see if it has an output_filename.
                if let Some(meta) = parse_markdown_front_matter(&path)? {
                    generated_files.push((dest_path, meta));
                }
            }
        }
    }

    Ok(generated_files)
}

/// Public function that creates the output folder structure,
/// converts/copies files, and **then** creates a single `content.md`
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

    // Add the content.md path if you want to save it, too
    all_md_paths.push(book_content_md_path);

    Ok(all_md_paths)
}
