use crate::error::{Result, SquashError};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tar::Archive;
use tempfile::TempDir;

/// Utility for extracting tar archives to temporary directories
pub struct TarExtractor {
    /// Temporary directory that holds extracted files
    pub temp_dir: TempDir,
    /// Path to the extracted content
    pub extracted_path: PathBuf,
}

impl TarExtractor {
    /// Extract a tar file to a temporary directory
    pub fn extract(tar_path: &Path) -> Result<Self> {
        let file = File::open(tar_path)?;
        let archive = Archive::new(BufReader::new(file));
        Self::extract_archive(archive)
    }

    /// Extract a gzipped tar file
    pub fn extract_gz(tar_gz_path: &Path) -> Result<Self> {
        let file = File::open(tar_gz_path)?;
        let gz_decoder = GzDecoder::new(BufReader::new(file));
        let archive = Archive::new(gz_decoder);
        Self::extract_archive(archive)
    }

    /// Common extraction logic for both regular and gzipped tar files
    fn extract_archive<R: std::io::Read>(mut archive: Archive<R>) -> Result<Self> {
        let temp_dir = TempDir::new()
            .map_err(SquashError::IoError)?;

        let extracted_path = temp_dir.path().to_path_buf();

        // Extract all files to the temporary directory
        archive.unpack(&extracted_path)?;

        Ok(TarExtractor {
            temp_dir,
            extracted_path,
        })
    }
    
    /// Get the path to an extracted file
    pub fn get_file_path(&self, filename: &str) -> PathBuf {
        self.extracted_path.join(filename)
    }
    
    /// Check if a file exists in the extracted directory
    pub fn file_exists(&self, filename: &str) -> bool {
        self.get_file_path(filename).exists()
    }
    
    /// Read a file from the extracted directory
    pub fn read_file(&self, filename: &str) -> Result<String> {
        let file_path = self.get_file_path(filename);
        std::fs::read_to_string(file_path)
            .map_err(SquashError::IoError)
    }
}

/// Utility for building tar archives from files and directories
pub struct TarBuilder {
    #[allow(dead_code)] // Needed to keep temporary directory alive
    temp_dir: TempDir,
    /// Path where files are staged before building the tar
    build_path: PathBuf,
}

impl TarBuilder {
    /// Create a new tar builder
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()
            .map_err(SquashError::IoError)?;
        
        let build_path = temp_dir.path().to_path_buf();
        
        Ok(TarBuilder {
            temp_dir,
            build_path,
        })
    }
    
    /// Add a file to the tar archive being built
    pub fn add_file(&self, filename: &str, content: &[u8]) -> Result<()> {
        let file_path = self.build_path.join(filename);
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(file_path, content)?;
        Ok(())
    }
    
    /// Add a directory to the tar archive
    pub fn add_directory(&self, dir_name: &str) -> Result<()> {
        let dir_path = self.build_path.join(dir_name);
        std::fs::create_dir_all(dir_path)?;
        Ok(())
    }
    
    /// Build the final tar file
    pub fn build(&self, output_path: &Path) -> Result<()> {
        let output_file = File::create(output_path)?;
        let mut archive = tar::Builder::new(output_file);
        
        // Add all files from the build directory to the archive
        archive.append_dir_all(".", &self.build_path)?;
        archive.finish()?;
        
        Ok(())
    }
    
    /// Get the build directory path
    pub fn build_path(&self) -> &Path {
        &self.build_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_tar_builder_creation() {
        let builder = TarBuilder::new().unwrap();
        assert!(builder.build_path().exists());
        assert!(builder.build_path().is_dir());
    }

    #[test]
    fn test_tar_builder_add_file() {
        let builder = TarBuilder::new().unwrap();
        let content = b"Hello, World!";

        builder.add_file("test.txt", content).unwrap();

        let file_path = builder.build_path().join("test.txt");
        assert!(file_path.exists());

        let read_content = fs::read(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_tar_builder_add_directory() {
        let builder = TarBuilder::new().unwrap();

        builder.add_directory("test_dir").unwrap();

        let dir_path = builder.build_path().join("test_dir");
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[test]
    fn test_tar_builder_nested_files() {
        let builder = TarBuilder::new().unwrap();
        let content = b"Nested file content";

        builder.add_file("nested/dir/file.txt", content).unwrap();

        let file_path = builder.build_path().join("nested/dir/file.txt");
        assert!(file_path.exists());

        let read_content = fs::read(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_tar_builder_build() {
        let builder = TarBuilder::new().unwrap();
        let content = b"Test content";

        builder.add_file("test.txt", content).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.tar");

        builder.build(&output_path).unwrap();
        assert!(output_path.exists());

        // Verify the tar file is not empty
        let metadata = fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 0);
    }
}
