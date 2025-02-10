use colored::Colorize;
use comrak::{markdown_to_html, ComrakOptions};
use once_cell::sync::Lazy;
use regex::Regex;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

/// Removes YAML front matter (metadata) from the beginning of a Markdown file.
/// Any block delimited by lines containing only `---` is removed.
fn strip_front_matter(markdown: &str) -> String {
    let mut lines = markdown.lines();
    if let Some(first_line) = lines.next() {
        if first_line.trim() == "---" {
            // Skip lines until we find the closing delimiter.
            for line in &mut lines {
                if line.trim() == "---" {
                    // Return the rest of the document.
                    return lines.collect::<Vec<&str>>().join("\n");
                }
            }
        }
    }
    markdown.to_string()
}

/// Unescape common HTML entities.
fn html_unescape(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}

/// Global lazy-loaded SyntaxSet and ThemeSet for code highlighting.
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| ThemeSet::load_defaults());

/// Searches for code blocks in the HTML produced by Comrak and replaces them with syntax‑highlighted HTML.
///
/// If the code block's language is "mermaid", the function will output the code as a Mermaid diagram
/// by simply wrapping it in `<pre class="mermaid"> ... </pre>`.
fn highlight_code_blocks(html: &str) -> String {
    // Modified regex:
    // Matches: <pre><code class=" ... language-(?:{.)?([a-zA-Z0-9_+\-]+)(?:})? ..."> ... </code></pre>
    let re = Regex::new(
        r#"(?s)<pre><code class="[^"]*language-(?:\{\.)?([a-zA-Z0-9_+\-]+)(?:\})?[^"]*">(.*?)</code></pre>"#
    ).unwrap();

    re.replace_all(html, |caps: &regex::Captures| {
        let lang = caps.get(1).unwrap().as_str();
        let code_html_escaped = caps.get(2).unwrap().as_str();
        let code = html_unescape(code_html_escaped);

        if lang == "mermaid" {
            // For Mermaid, simply output the raw code inside a <pre> with class "mermaid".
            format!("<pre class=\"mermaid\">{}</pre>", code)
        } else {
            // Look up the syntax by token; default to plain text if not found.
            let syntax = SYNTAX_SET
                .find_syntax_by_token(lang)
                .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
            // Use the "InspiredGitHub" theme if available; otherwise fallback.
            let theme = THEME_SET
                .themes
                .get("InspiredGitHub")
                .unwrap_or_else(|| &THEME_SET.themes["base16-ocean.dark"]);

            // Generate highlighted HTML. This returns a complete <pre> block with inline styles.
            match highlighted_html_for_string(&code, &SYNTAX_SET, syntax, theme) {
                Ok(highlighted) => highlighted,
                Err(_) => caps.get(0).unwrap().as_str().to_string(),
            }
        }
    })
    .to_string()
}

/// Generates an HTML file from a Markdown file:
/// 1. Reads the Markdown file and strips YAML front matter.
/// 2. Converts the Markdown to HTML using Comrak.
/// 3. Applies syntax highlighting to code blocks (or simply outputs Mermaid blocks).
/// 4. Wraps the content in a complete HTML document that inlines your CSS.
/// 5. Optionally injects a local Mermaid.js script if a path is provided.
/// 6. Writes the output to the specified path.
pub fn generate_html_from_markdown(
    input_path: &str,
    output_path: &str,
    css_path: &str,
    mermaid_js_path: Option<&str>,
) -> io::Result<()> {
    // Read and clean the Markdown.
    let markdown_content = fs::read_to_string(input_path)?;
    let cleaned_markdown = strip_front_matter(&markdown_content);

    // Set up Comrak options (enable useful extensions).
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.strikethrough = true;

    // Convert Markdown to HTML.
    let html_body_raw = markdown_to_html(&cleaned_markdown, &options);
    // Replace plain code blocks with highlighted versions or Mermaid blocks.
    let html_body = highlight_code_blocks(&html_body_raw);

    // Read your custom CSS (or use an empty string if the file can’t be read).
    let css_content = fs::read_to_string(css_path).unwrap_or_default();

    // Build the complete HTML document.
    let complete_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Documentation</title>
  <style>
    /* Inlined custom CSS */
    {}
  </style>
</head>
<body>
  <div class="container">
    {}
  </div>
</body>
</html>"#,
        css_content, html_body
    );

    // Write the output file.
    fs::write(output_path, complete_html)?;

    // If a Mermaid.js file is provided, inject it and clean up the Mermaid code blocks.
    if let Some(mermaid_js_path) = mermaid_js_path {
        inject_mermaid_script(output_path, mermaid_js_path)?;
        clean_mermaid_code_tags(output_path)?;
    }

    println!(
        "{} Generated HTML from {} to {}",
        "✔".green(),
        input_path,
        output_path
    );
    Ok(())
}

/// Injects the contents of the local Mermaid.js file into the HTML file.
/// This function reads the Mermaid.js file, wraps it in a <script> tag, and inserts it
/// just before the closing </body> tag.
fn inject_mermaid_script(html_file_path: &str, mermaid_js_path: &str) -> io::Result<()> {
    let mermaid_script_content = fs::read_to_string(mermaid_js_path)?;
    let mermaid_script = format!(
        r#"
<script type="module">
{}
mermaid.initialize({{ startOnLoad: true }});
</script>
"#,
        mermaid_script_content
    );

    let mut html_content = fs::read_to_string(html_file_path)?;
    if let Some(body_end) = html_content.find("</body>") {
        html_content.insert_str(body_end, &mermaid_script);
    } else {
        html_content.push_str(&mermaid_script);
    }

    fs::write(html_file_path, html_content)?;
    Ok(())
}

/// Removes the <code> tags inside <pre class="mermaid"> blocks.
/// This cleans up the HTML generated when Mermaid diagrams are processed.
fn clean_mermaid_code_tags(html_file_path: &str) -> io::Result<()> {
    let mut html_content = fs::read_to_string(html_file_path)?;
    let re = Regex::new(r#"<pre class="mermaid"><code>(?s)(.*?)</code></pre>"#).unwrap();
    html_content = re
        .replace_all(&html_content, r#"<pre class="mermaid">$1</pre>"#)
        .to_string();
    fs::write(html_file_path, html_content)?;
    Ok(())
}

/// Recursively processes all Markdown files in a folder (and subfolders) and creates corresponding HTML files.
/// Also creates a log file listing all generated HTML file paths.
/// The `mermaid_js_path` parameter is forwarded to enable Mermaid.js injection.
pub fn translate_markdown_folder(
    folder_path: &str,
    doc_folder: &str,
    css_path: &str,
    mermaid_js_path: Option<&str>,
) -> io::Result<()> {
    let mut html_paths: Vec<String> = Vec::new();
    translate_markdown_folder_internal(
        folder_path,
        doc_folder,
        css_path,
        mermaid_js_path,
        &mut html_paths,
    )?;

    let output_log = PathBuf::from(doc_folder).join("created_markdown_files.txt");
    let mut file = File::create(&output_log)?;
    for path in html_paths {
        writeln!(file, "{}", path)?;
    }
    Ok(())
}

/// Internal helper to recursively process folders.
fn translate_markdown_folder_internal(
    folder_path: &str,
    doc_folder: &str,
    css_path: &str,
    mermaid_js_path: Option<&str>,
    html_paths: &mut Vec<String>,
) -> io::Result<()> {
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let sub_doc_folder = PathBuf::from(doc_folder).join(
                path.file_name()
                    .expect("Directory should have a valid name"),
            );
            fs::create_dir_all(&sub_doc_folder)?;
            translate_markdown_folder_internal(
                path.to_str().unwrap(),
                sub_doc_folder.to_str().unwrap(),
                css_path,
                mermaid_js_path,
                html_paths,
            )?;
        } else if path.is_file()
            && path
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("md"))
                .unwrap_or(false)
        {
            let base_name = path.file_stem().unwrap().to_str().unwrap();
            // Output file naming can be adjusted as needed.
            let html_output_path = PathBuf::from(doc_folder).join(format!("{}.html", base_name));

            if let Err(e) = generate_html_from_markdown(
                path.to_str().unwrap(),
                html_output_path.to_str().unwrap(),
                css_path,
                mermaid_js_path,
            ) {
                eprintln!(
                    "{} Error generating HTML for {}: {}",
                    "!".red(),
                    path.display(),
                    e
                );
            } else {
                html_paths.push(html_output_path.to_str().unwrap().to_string());
            }
        }
    }
    Ok(())
}
