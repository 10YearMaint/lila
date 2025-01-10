use regex::Regex;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

/// Generates HTML from a markdown file using Pandoc and saves it to the specified output path.
/// Also injects the Mermaid.js script for rendering diagrams and removes unnecessary <code> tags.
pub fn generate_html_from_markdown(
    input_path: &str,
    output_path: &str,
    css_path: &str,
    mermaid_js_path: Option<&str>,
) -> io::Result<()> {
    let mut cmd = Command::new("pandoc");
    cmd.arg("--standalone")
        .arg("--to=html")
        .arg("--css")
        .arg(css_path)
        .arg("--output")
        .arg(output_path)
        .arg(input_path);

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("pandoc error: {}", String::from_utf8_lossy(&output.stderr)),
        ));
    }

    // If Mermaid.js injection is enabled, inject the script and clean tags
    if let Some(mermaid_js_path) = mermaid_js_path {
        inject_mermaid_script(output_path, mermaid_js_path)?;
        clean_mermaid_code_tags(output_path)?;
    }

    println!("Generated HTML from {} to {}", input_path, output_path);
    Ok(())
}

/// Injects the contents of the local Mermaid.js file into the HTML file.
fn inject_mermaid_script(html_file_path: &str, mermaid_js_path: &str) -> io::Result<()> {
    // Read the contents of the local Mermaid.js file
    let mermaid_script_content = fs::read_to_string(mermaid_js_path)?;

    // Create the script tag with the Mermaid.js content
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
        // If </body> tag is not found, append the script at the end
        html_content.push_str(&mermaid_script);
    }

    let mut file = File::create(html_file_path)?;
    file.write_all(html_content.as_bytes())?;
    Ok(())
}

/// Removes the <code> tags inside <pre class="mermaid"> blocks.
fn clean_mermaid_code_tags(html_file_path: &str) -> io::Result<()> {
    let mut html_content = fs::read_to_string(html_file_path)?;

    // Regular expression to match <pre class="mermaid"><code>...</code></pre> with multiline support
    let re = Regex::new(r#"<pre class="mermaid"><code>(?s)(.*?)</code></pre>"#).unwrap();

    // Replace with <pre class="mermaid">...</pre>
    html_content = re
        .replace_all(&html_content, r#"<pre class="mermaid">$1</pre>"#)
        .to_string();

    let mut file = File::create(html_file_path)?;
    file.write_all(html_content.as_bytes())?;
    Ok(())
}

/// Translates all markdown files in a folder to HTML, applying the specified CSS and optionally Mermaid.js files.
pub fn translate_markdown_folder(
    folder_path: &str,
    doc_folder: &str,
    css_path: &str,
    mermaid_path: Option<&str>,
) -> io::Result<()> {
    let mut html_paths: Vec<String> = Vec::new();
    translate_markdown_folder_internal(
        folder_path,
        doc_folder,
        css_path,
        mermaid_path,
        &mut html_paths,
    )?;

    // Write HTML file paths to a text file
    let output_path = PathBuf::from(doc_folder).join("created_html_files.txt");
    let mut file = std::fs::File::create(&output_path)?;
    for path in html_paths {
        writeln!(file, "{}", path)?;
    }
    Ok(())
}

/// Internal function to handle the recursive translation of markdown files in subfolders.
fn translate_markdown_folder_internal(
    folder_path: &str,
    doc_folder: &str,
    css_path: &str,
    mermaid_path: Option<&str>,
    html_paths: &mut Vec<String>,
) -> io::Result<()> {
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let sub_doc_folder = PathBuf::from(doc_folder).join(path.file_name().unwrap());
            fs::create_dir_all(&sub_doc_folder)?;
            translate_markdown_folder_internal(
                path.to_str().unwrap(),
                sub_doc_folder.to_str().unwrap(),
                css_path,
                mermaid_path,
                html_paths,
            )?;
        } else if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            let base_name = path.file_stem().unwrap().to_str().unwrap();
            let html_output_path =
                PathBuf::from(doc_folder).join(format!("{}_combined.html", base_name));
            if let Err(e) = generate_html_from_markdown(
                path.to_str().unwrap(),
                html_output_path.to_str().unwrap(),
                css_path,
                mermaid_path,
            ) {
                eprintln!("Error generating HTML for {}: {}", path.display(), e);
            } else {
                html_paths.push(html_output_path.to_str().unwrap().to_string());
            }
        }
    }
    Ok(())
}
