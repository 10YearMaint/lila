pub mod edit;
pub mod init;
pub mod remove;
pub mod render;
pub mod save;
pub mod tangle;
pub mod weave;

use clap::{Parser, Subcommand};

const HELP_TEMPLATE: &str = "\
{about}

Usage: {usage}

These are common lila commands used in various situations:

Start a new project:
    init         Initialize lila environment

Working with code:
    tangle       Extract pure source code from Markdown files.
    weave        Embed source code files back into Markdown format.
    edit         Auto-format code blocks in Markdown

Code Literat:
    render       Convert Markdown files with embedded code into HTML
    server       Start the AI Server for chatting with your rendered book about their underlying Markdown files

Project management:
    save         Save the Markdown code into a SQLite database
    rm           Remove files created by `tangle` and `render`. Use `-a` to remove all output folders

{after-help}";

#[derive(Parser, Debug)]
#[command(
    version,
    about = "lila: A CLI tool for code extraction, markdown2html translation, and saving metadata.",
    help_template = HELP_TEMPLATE
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize lila environment
    Init,

    /// Extract pure source code from Markdown files.
    Tangle {
        /// Specify a Markdown file to extract code from. Cannot be used with --folder.
        #[arg(short, long, value_name = "FILE", conflicts_with = "folder")]
        file: Option<String>,
        /// Specify a directory containing Markdown files to extract code from. Cannot be used with --file.
        #[arg(short, long, value_name = "FOLDER", conflicts_with = "file")]
        folder: Option<String>,
        /// Specify the output directory where extracted code will be saved.
        #[arg(short, long, value_name = "OUTPUT_DIR")]
        output: Option<String>,
        /// Specify a protocol (e.g., AImM) for special handling of extracted files.
        #[arg(short, long, value_name = "PROTOCOL")]
        protocol: Option<String>,
    },

    /// Embed source code files back into Markdown format.
    Weave {
        /// Specify a source code file to embed into Markdown. Cannot be used with --folder.
        #[arg(short, long, value_name = "FILE", conflicts_with = "folder")]
        file: Option<String>,
        /// Specify a directory containing source code files to embed into Markdown. Cannot be used with --file.
        #[arg(short, long, value_name = "FOLDER", conflicts_with = "file")]
        folder: Option<String>,
        /// Specify the output directory for the resulting Markdown files.
        #[arg(short, long, value_name = "OUTPUT_DIR")]
        output: Option<String>,
    },

    /// Auto-format code blocks (Python, Rust, etc.) in a Markdown file or folder.
    Edit {
        /// Specify a single Markdown file (conflicts with folder)
        #[arg(short, long, conflicts_with = "folder")]
        file: Option<String>,
        /// Specify a folder containing Markdown files (conflicts with file)
        #[arg(short, long, conflicts_with = "file")]
        folder: Option<String>,
    },

    /// Convert Markdown files with embedded code into HTML
    Render {
        /// Folder containing Markdown files to be rendered
        #[arg(short, long)]
        folder: String,
        /// Optional: Output folder for rendered files (default: ~/.lila/<project_name>/doc)
        #[arg(short, long)]
        output: Option<String>,
        /// Optional: Custom CSS file for the output HTML
        #[arg(short, long)]
        css: Option<String>,
        /// Optional: Mermaid.js file for diagram rendering
        #[arg(short, long)]
        mermaid: Option<String>,
        /// Optional: Mathjax.js for latex rendering
        #[arg(short, long)]
        mathjax: Option<String>,
        /// Optional: Disable Mermaid.js injection
        #[arg(long, default_value_t = false)]
        disable_mermaid: bool,
        /// Optional: Replace markdown (.md) links with HTML (.html) links (for book-style indexes)
        #[arg(long, default_value_t = false)]
        book_render: bool,
    },

    /// Save the weaved code and metadata into a SQLite database.
    Save {
        /// Optional path to the SQLite database
        #[arg(short, long)]
        db: Option<String>,

        /// Specify the input directory of the Markdown files.
        #[arg(short, long, value_name = "INPUT_DIR")]
        input: Option<String>,
    },

    /// Remove files created by `tangle` and `render`. Use `-a` to remove all output folders.
    Rm {
        /// Remove all files from the output folder, including other projects in .lila
        #[arg(short, long)]
        all: bool,
        /// Output folder to remove (default: ~/.lila/<project_name>)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Start the AI Server for chatting with your rendered book
    Server,
}
