use colored::Colorize;
use sysinfo::System;

pub fn run_recommend() {
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
    let total_memory_kb = sys.total_memory();
    let total_memory_gb = total_memory_kb as f64 / 1_048_576.0;

    // Display system information
    println!("System Recommendation:");
    println!("------------------------");
    println!("CPU: {} cores ({})", cpu_count, cpu_name);
    println!("Total Memory: {:.2} GB", total_memory_gb);

    // Define heuristic thresholds
    let min_cpu_for_7b = 8;
    let min_memory_for_7b = 16.0; // GB

    // Determine recommendation
    let recommendation = if cpu_count >= min_cpu_for_7b && total_memory_gb >= min_memory_for_7b {
        "3B model".green()
    } else {
        "1B model".yellow()
    };

    println!("\nRecommended AI Model: {}", recommendation);

    // Additional Suggestions
    if recommendation.to_string().contains("3B") {
        println!("You have a powerful system! You can efficiently run the 3B model.");
    } else {
        println!("Your system is suitable for a smaller than 3B model. Consider upgrading CPU or RAM for better performance.");
    }
}
