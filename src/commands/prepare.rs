use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

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
