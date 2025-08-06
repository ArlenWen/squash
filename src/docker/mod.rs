//! Docker image manipulation utilities.
//!
//! This module provides functionality for working with Docker images,
//! including parsing, layer manipulation, and tar archive handling.

/// Docker image parsing and manipulation
pub mod image;
/// Tar archive extraction and building utilities
pub mod tar;
/// Layer merging and squashing functionality
pub mod layer;

pub use image::*;
pub use tar::*;
pub use layer::*;
