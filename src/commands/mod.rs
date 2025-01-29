pub mod auto;
pub mod chat;
pub mod convert;
pub mod extract;
pub mod remove;
pub mod save;
pub mod translate;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about = "lila: A CLI tool for code extraction, markdown2html translation, and saving metadata.", long_about = None)]
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

    /// Convert code files (.py, .rs, .cpp, etc.) back into Markdown
    Convert {
        /// Specify a code file to convert (conflicts with folder)
        #[arg(short, long, conflicts_with = "folder")]
        file: Option<String>,

        /// Specify a folder containing code files (conflicts with file)
        #[arg(short, long, conflicts_with = "file")]
        folder: Option<String>,

        /// Optional: Specify the output folder for the resulting Markdown
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Auto-format code blocks (Python, Rust, etc.) in a Markdown file *or* folder
    Auto {
        /// Specify a single Markdown file (conflicts with folder)
        #[arg(short, long, conflicts_with = "folder")]
        file: Option<String>,

        /// Specify a folder containing Markdown files (conflicts with file)
        #[arg(short, long, conflicts_with = "file")]
        folder: Option<String>,
    },

    /// Translate Markdown files into HTML using Pandoc
    Translate {
        /// Folder containing Markdown files to be translated
        #[arg(short, long)]
        folder: String,

        /// Optional: Output folder for translated files (default: ~/.lila/<project_name>/doc)
        #[arg(short, long)]
        output: Option<String>,

        /// Optional: Custom CSS file for the output HTML
        #[arg(short, long)]
        css: Option<String>,

        /// Optional: Mermaid.js file for diagram rendering
        #[arg(short, long)]
        mermaid: Option<String>,

        /// Optional: Disable Mermaid.js injection
        #[arg(long, default_value_t = false)]
        disable_mermaid: bool,
    },

    /// Save the extracted code and HTML metadata into a SQLite database
    Save {
        /// Optional: Path to the SQLite database (default: <doc_pure>/lila.db)
        #[arg(short, long)]
        db: Option<String>,
    },

    /// Remove files created by `extract` and `translate`. Use `-a` to remove all output folders.
    Rm {
        /// Remove all files from the output folder, including other projects in .lila
        #[arg(short, long)]
        all: bool,

        /// Output folder to remove (default: ~/.lila/<project_name>)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Chat subcommand
    Chat {
        /// Run on CPU instead of GPU
        #[arg(long)]
        cpu: bool,

        /// Enable tracing (generates a trace-timestamp.json file)
        #[arg(long)]
        tracing: bool,

        /// Display verbose tokenization info
        #[arg(long)]
        verbose_prompt: bool,

        /// The prompt text to feed to the model
        #[arg(long)]
        prompt: Option<String>,

        /// The temperature used to generate samples
        #[arg(long)]
        temperature: Option<f64>,

        /// Nucleus sampling probability cutoff
        #[arg(long)]
        top_p: Option<f64>,

        /// The random seed
        #[arg(long, default_value_t = 299792458)]
        seed: u64,

        /// The length of the sample to generate (in tokens)
        #[arg(long, short = 'n', default_value_t = 5000)]
        sample_len: usize,

        #[arg(long)]
        model_id: Option<String>,

        #[arg(long, default_value = "2")]
        model: String, // or an enum if you prefer

        #[arg(long)]
        revision: Option<String>,

        #[arg(long)]
        weight_file: Option<String>,

        #[arg(long)]
        tokenizer: Option<String>,

        #[arg(long)]
        quantized: bool,

        /// Penalty to be applied for repeating tokens, 1. means no penalty.
        #[arg(long, default_value_t = 1.1)]
        repeat_penalty: f32,

        /// The context size to consider for the repeat penalty.
        #[arg(long, default_value_t = 64)]
        repeat_last_n: usize,

        /// The dtype, e.g. f32, bf16, or f16
        #[arg(long)]
        dtype: Option<String>,
    },
}
