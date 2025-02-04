pub mod chat;
pub mod edit;
pub mod init;
pub mod remove;
pub mod render;
pub mod save;
pub mod tangle;
pub mod weave;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about = "lila: A CLI tool for code extraction, markdown2html translation, and saving metadata.", long_about = None)]
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

        /// Specify the output directory where extracted code will be saved. Defaults to the current directory.
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

        /// Specify the output directory for the resulting Markdown files. Defaults to the current directory.
        #[arg(short, long, value_name = "OUTPUT_DIR")]
        output: Option<String>,
    },

    /// Auto-format code blocks (Python, Rust, etc.) in a Markdown file *or* folder
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

        /// Optional: Disable Mermaid.js injection
        #[arg(long, default_value_t = false)]
        disable_mermaid: bool,
    },

    /// Save the rendered code and HTML metadata into a SQLite database
    Save {
        /// Optional: Path to the SQLite database (default: <doc_pure>/lila.db)
        #[arg(short, long)]
        db: Option<String>,
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

        /// Disable loading data from the DB
        #[arg(long, default_value_t = false)]
        no_db: bool,
    },
}
