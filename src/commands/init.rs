use colored::Colorize;
use dirs::home_dir;
use std::ffi::OsStr;
use std::fs::{create_dir_all, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use sysinfo::System;

/// Checks if a given command is available on the user's system
/// by attempting `command --version` (or another trivial arg).
fn check_program_availability(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .is_ok()
}

/// Updates or inserts a key-value pair into the `.env` file.
///
/// If the key is found, it replaces that line. Otherwise, it appends at the end.
fn update_env_value(key: &str, value: &str) -> io::Result<()> {
    let env_path = Path::new(".env");

    // If .env does not exist, create it.
    if !env_path.exists() {
        let mut file = File::create(env_path)?;
        writeln!(file, "# lila environment settings")?;
        writeln!(file, "{}={}", key, value)?;
        return Ok(());
    }

    // If .env exists, either update or append the key=value pair.
    let content = std::fs::read_to_string(env_path)?;
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut found = false;

    for line in &mut lines {
        if line.trim_start().starts_with(&format!("{}=", key)) {
            *line = format!("{}={}", key, value);
            found = true;
            break;
        }
    }

    if !found {
        lines.push(format!("{}={}", key, value));
    }

    // Rewrite .env
    let mut file = File::create(env_path)?;
    for line in lines {
        writeln!(file, "{}", line)?;
    }

    Ok(())
}

/// Gathers system info and recommends an AI model (1B or 3B).
/// If 3B is recommended, let the user choose between two 3B models
/// and write that choice into `.env`.
fn run_recommend() -> io::Result<()> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Gather CPU information
    let cpu_count = sys.cpus().len();
    let cpu_name = sys
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Gather memory information (in GB)
    let total_memory_kb = sys.total_memory(); // in KiB
    let total_memory_gb = total_memory_kb as f64 / 1_048_576.0; // KiB -> GiB

    // Display system information
    println!("\nSystem Recommendation:");
    println!("------------------------");
    println!("CPU: {} cores ({})", cpu_count, cpu_name);
    println!("Total Memory: {:.2} GB", total_memory_gb);

    // Define heuristic thresholds
    let min_cpu_for_3b = 8;
    let min_memory_for_3b = 16.0; // GB

    // Determine recommendation
    let recommendation =
        if cpu_count as u64 >= min_cpu_for_3b && total_memory_gb >= min_memory_for_3b {
            "3B model".green()
        } else {
            "1B model".yellow()
        };

    println!("\nRecommended AI Model: {}", recommendation);

    // Additional suggestions
    if recommendation.to_string().contains("3B") {
        println!("You have a powerful system! You can efficiently run the 3B model.\n");

        // Ask the user which 3B model they'd like to set in .env
        println!(
            "{}",
            "Which 3B model do you want to set as your default?".bold()
        );
        println!("1) microsoft/Phi-3.5-mini-instruct");
        println!("2) Qwen/Qwen2.5-Coder-3B-Instruct");

        print!("Enter 1 or 2: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        // Default to the first if invalid input
        let model_selected = match choice {
            "1" => "microsoft/Phi-3.5-mini-instruct",
            "2" => "Qwen/Qwen2.5-Coder-3B-Instruct",
            _ => {
                println!("Invalid choice, defaulting to 1.");
                "microsoft/Phi-3.5-mini-instruct"
            }
        };

        update_env_value("LILA_AI_MODEL", model_selected)?;
        println!(
            "{} {} {}",
            "Set".green(),
            "LILA_AI_MODEL=".yellow(),
            model_selected.green()
        );
    } else {
        println!("Your system is suitable for a smaller than 3B model. Consider upgrading CPU or RAM for better performance.\n");
    }

    Ok(())
}

/// Helper function to run `rustc --version` and extract the major.minor version.
/// Returns a string like "1.71" if successful.
fn get_rustc_version() -> Option<String> {
    let output = Command::new("rustc").arg("--version").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Example output: "rustc 1.71.0 (abc123 2023-10-05)"
    let version_token = stdout.split_whitespace().nth(1)?;
    let parts: Vec<&str> = version_token.split('.').collect();
    if parts.len() >= 2 {
        Some(format!("{}.{}", parts[0], parts[1]))
    } else {
        None
    }
}

/// Interactively creates a `Lila.toml` file with several sections:
/// - [project]: asks for context and deployment description
/// - [compliance]: added only if the user chooses to include compliance guidelines
/// - [ai_guidance]: always includes a fixed code_of_conduct
/// - [development]: detects the programming languages, operating system, and architecture
/// - [dependencies]: for example, if Rust is selected, attempts to parse Cargo.toml for dependencies
fn create_lila_toml() -> io::Result<()> {
    // 1. [project] section
    let mut project_context = String::new();
    println!("\nEnter the project context (e.g. \"Physics engine for tissue simulation\"):");
    io::stdin().read_line(&mut project_context)?;
    let project_context = {
        let trimmed = project_context.trim();
        if trimmed.is_empty() {
            "Default project context".to_string()
        } else {
            trimmed.to_string()
        }
    };

    let mut deployment = String::new();
    println!(
        "Enter the deployment description (e.g. \"on-premise with enterprise intranet-only\"):"
    );
    io::stdin().read_line(&mut deployment)?;
    let deployment = {
        let trimmed = deployment.trim();
        if trimmed.is_empty() {
            "on-premise with enterprise intranet-only".to_string()
        } else {
            trimmed.to_string()
        }
    };

    // 2. [compliance] section (optional)
    let mut compliance_input = String::new();
    println!("Do you have compliance guidelines to follow? (y/N):");
    io::stdout().flush()?;
    io::stdin().read_line(&mut compliance_input)?;
    let compliance_input = compliance_input.trim().to_lowercase();
    let compliance_section = if compliance_input == "y" || compliance_input == "yes" {
        // Ask for ISO guidelines
        let mut iso = String::new();
        println!("Enter ISO compliance guidelines separated by comma (e.g. ISO/IEC 22989:2022):");
        io::stdin().read_line(&mut iso)?;
        let iso: Vec<&str> = iso
            .trim()
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // Ask for BSI guidelines
        let mut bsi = String::new();
        println!(
            "Enter BSI compliance guidelines separated by comma (e.g. APP.6 Allgemeine Software):"
        );
        io::stdin().read_line(&mut bsi)?;
        let bsi: Vec<&str> = bsi
            .trim()
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // Format arrays for TOML
        let iso_array = format!(
            "[{}]",
            iso.iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let bsi_array = format!(
            "[{}]",
            bsi.iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ")
        );
        format!("[compliance]\niso = {}\nbsi = {}\n", iso_array, bsi_array)
    } else {
        String::new()
    };

    // 3. [ai_guidance] section (basic code_of_conduct is fixed)
    let code_of_conduct = r#"- Prioritize secure coding practices aligned with ISO/IEC 22989:2022 guidelines.
- Do not introduce external dependencies beyond those listed in [dependencies] if applicable.
- If uncertain about compliance requirements, refer to the relevant compliance references which the user has to provide you."#;

    // 4. [development] section
    // Ask for the programming languages used (we will auto-detect OS and architecture)
    let mut languages_input = String::new();
    println!("Enter the programming languages used in this project (comma separated, e.g. rust, python):");
    io::stdin().read_line(&mut languages_input)?;
    let languages: Vec<&str> = languages_input
        .trim()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // For each language, if "rust" is chosen, auto-detect the installed rustc version.
    let mut language_entries = Vec::new();
    for lang in languages.iter() {
        if lang.eq_ignore_ascii_case("rust") {
            let version = get_rustc_version().unwrap_or_else(|| "1.71".to_string());
            language_entries.push(format!("\"rust~={}\"", version));
        } else if lang.eq_ignore_ascii_case("python") {
            // TODO: add auto-detection here as well.
            language_entries.push("\"python~=3.10\"".to_string());
        } else {
            language_entries.push(format!("\"{}\"", lang));
        }
    }
    let languages_array = format!("[{}]", language_entries.join(", "));

    // Auto-detect operating system and architecture.
    let mut sys = System::new_all();
    sys.refresh_all();
    // Try using sysinfo for OS name and version; fall back to Rust constants.
    let os_name = System::name().unwrap_or_else(|| std::env::consts::OS.to_string());
    let os_version = System::os_version().unwrap_or_else(|| "".to_string());
    let operating_system = if os_version.is_empty() {
        os_name
    } else {
        format!("{} {}", os_name, os_version)
    };
    let os_array = format!("[\"{}\"]", operating_system);

    // For architecture, we use Rust's built-in constant.
    let architecture = std::env::consts::ARCH;
    let arch_array = format!("[\"{}\"]", architecture);

    // 5. [dependencies] section
    // We'll build two subsections: one for Python and one for Rust.
    let mut dependencies_rust = String::new();
    let mut dependencies_python = String::new();

    // If Rust is among the chosen languages, try to parse Cargo.toml
    if languages
        .iter()
        .any(|&lang| lang.eq_ignore_ascii_case("rust"))
    {
        let cargo_path = Path::new("Cargo.toml");
        if cargo_path.exists() {
            let cargo_content = std::fs::read_to_string(cargo_path)?;
            // Parse using the toml crate (make sure to add toml = "0.5" to your Cargo.toml dependencies)
            let cargo_toml: toml::Value =
                toml::from_str(&cargo_content).unwrap_or(toml::Value::Table(Default::default()));
            if let Some(deps) = cargo_toml.get("dependencies") {
                if let Some(deps_table) = deps.as_table() {
                    for (key, value) in deps_table {
                        // Format each dependency as: key = <value>
                        dependencies_rust.push_str(&format!("{} = {}\n", key, value));
                    }
                }
            }
        } else {
            println!("No Cargo.toml found in the current directory, skipping Rust dependencies extraction.");
        }
    }

    // If Python is chosen, use a default list (you might later extend this to auto-detect)
    if languages
        .iter()
        .any(|&lang| lang.eq_ignore_ascii_case("python"))
    {
        dependencies_python.push_str("");
    }

    // 6. Build the complete Lila.toml content
    let mut lila_toml = String::new();
    // [project] section
    lila_toml.push_str("[project]\n");
    lila_toml.push_str(&format!("context = \"{}\"\n", project_context));
    lila_toml.push_str(&format!("deployment = \"{}\"\n\n", deployment));
    // [compliance] section (if provided)
    if !compliance_section.is_empty() {
        lila_toml.push_str(&compliance_section);
        lila_toml.push('\n');
    }
    // [ai_guidance] section
    lila_toml.push_str("[ai_guidance]\n");
    lila_toml.push_str("code_of_conduct = \"\"\"\n");
    lila_toml.push_str(code_of_conduct);
    lila_toml.push_str("\n\"\"\"\n\n");
    // [development] section
    lila_toml.push_str("[development]\n");
    lila_toml.push_str(&format!("languages = {}\n", languages_array));
    lila_toml.push_str(&format!("operating_systems = {}\n", os_array));
    lila_toml.push_str(&format!("architecture = {}\n\n", arch_array));
    // [dependencies] section
    lila_toml.push_str("[dependencies]\n\n");
    if !dependencies_python.is_empty() {
        lila_toml.push_str("  [dependencies.python]\n");
        for line in dependencies_python.lines() {
            lila_toml.push_str("  ");
            lila_toml.push_str(line);
            lila_toml.push('\n');
        }
        lila_toml.push('\n');
    }
    if !dependencies_rust.is_empty() {
        lila_toml.push_str("  [dependencies.rust]\n");
        for line in dependencies_rust.lines() {
            lila_toml.push_str("  ");
            lila_toml.push_str(line);
            lila_toml.push('\n');
        }
        lila_toml.push('\n');
    }

    // Write Lila.toml to the current directory
    let mut file = File::create("Lila.toml")?;
    file.write_all(lila_toml.as_bytes())?;
    println!("\n{}", "Lila.toml created successfully.".bright_green());
    Ok(())
}

/// Initializes the project for Lila:
/// 1) Sets a default LILA_OUTPUT_PATH (i.e. ~/.lila/<project_name>)
/// 2) Checks for `black` / `rustfmt` and sets environment flags
/// 3) Runs AI model recommendation
/// 4) Creates a Lila.toml file for project configuration
pub fn init() -> io::Result<()> {
    println!("{}", "Welcome to lila init!".bright_green());
    println!("This will check for code formatters and record them in your .env file.\n");

    // 1) Set the default LILA_OUTPUT_PATH
    let home = home_dir().expect("Could not determine the home directory");
    let current_dir = std::env::current_dir()?;
    let project_name = current_dir
        .file_name()
        .unwrap_or_else(|| OsStr::new("default"))
        .to_string_lossy()
        .to_string();
    let lila_root = home.join(".lila");
    let default_root = lila_root.join(&project_name);

    // Give the user a chance to override or accept
    println!(
        "Default project output path is: {}\nPress ENTER to accept or type a different path:",
        default_root.display()
    );
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let path_input = input.trim();

    let final_path = if path_input.is_empty() {
        default_root
    } else {
        PathBuf::from(path_input)
    };

    // Ensure that the final_path (and parents) are created
    create_dir_all(&final_path)?;

    // Write LILA_OUTPUT_PATH to .env
    update_env_value("LILA_OUTPUT_PATH", &final_path.to_string_lossy())?;

    // 2) Check for black
    let black_installed = check_program_availability("black");
    let black_msg = if black_installed {
        "Detected 'black' on this system."
    } else {
        "Could NOT detect 'black' on this system."
    };
    println!("{}", black_msg.bright_yellow());
    update_env_value(
        "BLACK_INSTALLED",
        if black_installed { "true" } else { "false" },
    )?;

    // 2a) Check for rustfmt
    let rustfmt_installed = check_program_availability("rustfmt");
    let rustfmt_msg = if rustfmt_installed {
        "Detected 'rustfmt' on this system."
    } else {
        "Could NOT detect 'rustfmt' on this system."
    };
    println!("{}", rustfmt_msg.bright_yellow());
    update_env_value(
        "RUSTFMT_INSTALLED",
        if rustfmt_installed { "true" } else { "false" },
    )?;

    // 3) Run system-based recommendation for AI model
    run_recommend()?;

    // 4) Create Lila.toml configuration file
    println!(
        "\n{}",
        "Now let’s configure your project via Lila.toml.".bright_green()
    );
    create_lila_toml()?;

    println!(
        "\n{}",
        "Done! Your .env and Lila.toml files have been updated.".bright_green()
    );
    println!("You can re-run `lila init` anytime if you install new formatters or want to update your configuration.\n");
    Ok(())
}
