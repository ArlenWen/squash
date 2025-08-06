use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "squash")]
#[command(about = "A Docker image layer squashing tool")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Squash Docker image layers
    Squash {
        /// Source image (name:tag or file path)
        #[arg(short, long)]
        source: String,

        /// Output file path (required if not using --load)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Load result into Docker with name:tag
        #[arg(long)]
        load: Option<String>,

        /// Temporary directory for intermediate files
        #[arg(short, long)]
        temp_dir: Option<PathBuf>,

        /// Layer specification: number (merge latest n layers) or layer ID
        #[arg(short, long)]
        layers: String,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
