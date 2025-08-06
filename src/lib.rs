//! # Squash - Docker Image Layer Squashing Tool
//!
//! A command-line tool for squashing Docker image layers, written in Rust.
//!
//! ## Features
//!
//! - **Multiple Input Sources**: Accept Docker image name:tag or exported/saved image files
//! - **Flexible Output**: Save to file or load directly into Docker with specified name:tag
//! - **Layer Merging Options**:
//!   - By count: merge latest n layers into one
//!   - By layer ID: merge from specific layer ID to latest layer
//! - **Temporary Directory Support**: Configure where intermediate files are stored
//! - **Verbose Output**: Detailed logging of operations
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use squash::{cli::Cli, docker::DockerImage};
//! use clap::Parser;
//!
//! // Parse command line arguments
//! let cli = Cli::parse();
//!
//! // Load and process Docker image
//! // (This is a simplified example - see main.rs for complete implementation)
//! ```

/// Command line interface definitions
pub mod cli;
/// Docker image manipulation utilities
pub mod docker;
/// Error types and handling
pub mod error;

pub use cli::*;
pub use error::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI parsing works correctly
        use clap::Parser;

        let args = vec![
            "squash",
            "squash",
            "--source", "test.tar",
            "--output", "output.tar",
            "--layers", "2",
            "--verbose"
        ];

        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Squash { source, output, layers, verbose, .. } => {
                assert_eq!(source, "test.tar");
                assert_eq!(output.unwrap().to_str().unwrap(), "output.tar");
                assert_eq!(layers, "2");
                assert!(verbose);
            }
        }
    }

    #[test]
    fn test_error_types() {
        use std::io;

        // Test error conversion
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let squash_error: SquashError = io_error.into();

        match squash_error {
            SquashError::IoError(_) => {}, // Expected
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_cli_parsing_with_load() {
        use clap::Parser;

        let args = vec![
            "squash",
            "squash",
            "--source", "nginx:latest",
            "--load", "nginx:squashed",
            "--layers", "3",
        ];

        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Squash { source, load, layers, .. } => {
                assert_eq!(source, "nginx:latest");
                assert_eq!(load.unwrap(), "nginx:squashed");
                assert_eq!(layers, "3");
            }
        }
    }

    #[test]
    fn test_cli_parsing_with_temp_dir() {
        use clap::Parser;

        let args = vec![
            "squash",
            "squash",
            "--source", "test.tar",
            "--output", "output.tar",
            "--layers", "2",
            "--temp-dir", "/tmp/squash",
        ];

        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Squash { source, output, layers, temp_dir, .. } => {
                assert_eq!(source, "test.tar");
                assert_eq!(output.unwrap().to_str().unwrap(), "output.tar");
                assert_eq!(layers, "2");
                assert_eq!(temp_dir.unwrap().to_str().unwrap(), "/tmp/squash");
            }
        }
    }
}
