use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn process_protocol_aimm(app_folder: &Path) -> io::Result<()> {
    let mut folders_to_process = Vec::new();

    for entry in WalkDir::new(app_folder).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            if path.ends_with("private") || path.ends_with("public") {
                folders_to_process.push(path.to_path_buf());
            }
        }
    }

    let mut processed_parents = std::collections::HashSet::new();

    for folder in folders_to_process {
        let parent = folder.parent().unwrap().to_path_buf();
        if processed_parents.contains(&parent) {
            continue;
        }

        let private_folder = parent.join("private");
        let public_folder = parent.join("public");
        let src_folder = parent.join("src");

        let mut sub_folders = vec![];
        if private_folder.exists() && private_folder.is_dir() {
            sub_folders.push(private_folder.clone());
        }
        if public_folder.exists() && public_folder.is_dir() {
            sub_folders.push(public_folder.clone());
        }

        if !sub_folders.is_empty() {
            println!("Combining folders into {:?}", src_folder);
            combine_folders(&sub_folders, &src_folder)?;

            if private_folder.exists() {
                fs::remove_dir_all(&private_folder)?;
            }
            if public_folder.exists() {
                fs::remove_dir_all(&public_folder)?;
            }

            processed_parents.insert(parent);
        }
    }

    Ok(())
}

pub fn combine_folders(folders: &[PathBuf], dest_folder: &PathBuf) -> io::Result<()> {
    for folder in folders {
        if folder.exists() && folder.is_dir() {
            println!("Processing folder: {:?}", folder);
            for entry in fs::read_dir(&folder)? {
                let entry = entry?;
                let entry_path = entry.path();
                let dest_path = dest_folder.join(entry_path.file_name().unwrap());

                if entry_path.is_dir() {
                    combine_folders(&[entry_path], &dest_path)?;
                } else {
                    fs::create_dir_all(dest_folder)?;
                    fs::copy(&entry_path, &dest_path)?;
                    println!("Copied file to {:?}", dest_path);
                }
            }
        } else {
            println!("Folder does not exist or is not a directory: {:?}", folder);
        }
    }
    Ok(())
}
