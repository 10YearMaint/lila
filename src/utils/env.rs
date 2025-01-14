use std::process::Command;

pub fn ensure_pandoc_installed() -> bool {
    let output = Command::new("pandoc").arg("--version").output();

    match output {
        Ok(output) if output.status.success() => true,
        _ => false,
    }
}
