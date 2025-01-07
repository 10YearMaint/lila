pub mod extract;
pub mod models;
pub mod save;
pub mod translate;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about = "Leli: A CLI tool for code extraction, markdown2html translation, and saving metadata.", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Extract literated code from a Markdown file or folder containing Markdown files
    Extract {
        /// Specify a Markdown file to extract code from (conflicts with folder)
        #[arg(short, long, conflicts_with = "folder")]
        file: Option<String>,

        /// Specify a folder containing Markdown files to extract code from (conflicts with file)
        #[arg(short, long, conflicts_with = "file")]
        folder: Option<String>,

        /// Optional: Specify the output folder where extracted code will be saved
        #[arg(short, long)]
        output: Option<String>,

        /// Optional: Specify a protocol (e.g., AImM) for special handling of extracted files
        #[arg(short, long)]
        protocol: Option<String>,
    },

    /// Translate Markdown files into HTML using Pandoc
    Translate {
        /// Folder containing Markdown files to be translated
        #[arg(short, long)]
        folder: String,

        /// Optional: Output folder for translated files (default: ~/.leli/<project_name>/doc)
        #[arg(short, long)]
        output: Option<String>,

        /// Optional: custom CSS file for the output HTML
        #[arg(short, long)]
        css: Option<String>,

        /// Optional: Mermaid.js file for diagram rendering
        #[arg(short, long)]
        mermaid: Option<String>,
    },

    /// Save the extracted code and HTML metadata into a SQLite database
    Save {
        /// File listing the created HTML files to be saved into the database
        #[arg(short, long)]
        file: String,

        /// Path to the SQLite database
        #[arg(short, long)]
        db: String,
    },
}
