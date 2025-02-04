use colored::Colorize;
use dirs::home_dir;
use std::ffi::OsStr;
use std::fs::File;
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
        // Flush stdout before reading line
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

/// Initializes the project for Lila:
/// 1) Sets a default LILA_OUTPUT_PATH (i.e. ~/.lila/<project_name>)
/// 2) Checks for `black` / `rustfmt` and sets environment flags
/// 3) Runs AI model recommendation
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

    // Write LILA_OUTPUT_PATH to .env
    update_env_value("LILA_OUTPUT_PATH", &final_path.to_string_lossy())?;

    // 2) Check black
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

    // 2a) Check rustfmt
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

    println!("\n{}", "Done! Your .env file was updated.".bright_green());
    println!("You can re-run `lila init` anytime if you install new formatters or want to update your AI model.\n");
    Ok(())
}
