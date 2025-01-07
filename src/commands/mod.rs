pub mod extract;
pub mod models;
pub mod save;
pub mod translate;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Extract {
        #[arg(short, long, conflicts_with = "folder")]
        file: Option<String>,
        #[arg(short, long, conflicts_with = "file")]
        folder: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long)]
        protocol: Option<String>,
    },
    Translate {
        #[arg(short, long)]
        folder: String,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long)]
        css: Option<String>,
        #[arg(short, long)]
        mermaid: Option<String>,
    },
    Save {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        db: String,
    },
}
