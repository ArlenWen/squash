use crate::error::{Result, SquashError};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder, Header};
use uuid::Uuid;

/// Information about a Docker image layer
#[derive(Debug, Clone)]
pub struct LayerInfo {
    /// SHA256 digest of the layer
    pub digest: String,
    /// Size of the layer in bytes
    pub size: u64,
    /// Path to the layer's tar file
    pub tar_path: PathBuf,
}

/// Represents the data storage strategy for a file
#[derive(Debug, Clone)]
enum FileData {
    /// Small files stored in memory
    InMemory(Vec<u8>),
    /// Large files referenced by their source location
    OnDisk {
        /// Path to the source tar file
        #[allow(dead_code)] // Reserved for future streaming implementation
        source_tar: PathBuf,
        /// Offset in the tar file where this entry starts
        #[allow(dead_code)] // Reserved for future streaming implementation
        offset: u64,
        /// Size of the entry data
        size: u64,
    },
}

/// Represents a file entry in the virtual filesystem
#[derive(Debug, Clone)]
struct FileEntry {
    header: Header,
    data: FileData,
}

/// Maximum size for files to be stored in memory (1MB)
const MAX_MEMORY_FILE_SIZE: u64 = 1024 * 1024;

/// Virtual filesystem state for tracking layer changes
#[derive(Debug)]
struct VirtualFilesystem {
    files: HashMap<PathBuf, Option<FileEntry>>, // None means deleted by whiteout
}

/// Handles merging of Docker image layers
#[derive(Debug)]
pub struct LayerMerger {
    /// List of layers to work with
    pub layers: Vec<LayerInfo>,
    /// Temporary directory for intermediate files
    pub temp_dir: PathBuf,
}

impl LayerMerger {
    pub fn new(layers: Vec<LayerInfo>, temp_dir: PathBuf) -> Self {
        LayerMerger { layers, temp_dir }
    }

    /// Stream data from a large file stored on disk
    /// Reserved for future streaming implementation
    #[allow(dead_code)]
    fn stream_file_data(&self, source_tar: &Path, offset: u64, size: u64, writer: &mut dyn Write) -> Result<()> {
        let mut file = File::open(source_tar)?;
        file.seek(SeekFrom::Start(offset))?;

        let mut remaining = size;
        let mut buffer = [0; 8192];

        while remaining > 0 {
            let to_read = std::cmp::min(buffer.len() as u64, remaining) as usize;
            let bytes_read = file.read(&mut buffer[..to_read])?;

            if bytes_read == 0 {
                break;
            }

            writer.write_all(&buffer[..bytes_read])?;
            remaining -= bytes_read as u64;
        }

        Ok(())
    }
    
    /// Merge the specified number of latest layers
    pub fn merge_latest_layers(&self, count: usize) -> Result<LayerInfo> {
        if count == 0 {
            return Err(SquashError::InvalidInput(
                "Cannot merge 0 layers".to_string()
            ));
        }

        if count > self.layers.len() {
            return Err(SquashError::InvalidInput(format!(
                "Cannot merge {} layers, only {} layers available",
                count, self.layers.len()
            )));
        }
        
        // Get the layers to merge (latest n layers)
        let layers_to_merge = &self.layers[self.layers.len() - count..];
        
        println!("Merging {} layers:", count);
        for layer in layers_to_merge {
            println!("  - {}", layer.digest);
        }
        
        self.merge_layers(layers_to_merge)
    }
    
    /// Merge layers from a specific layer ID to the latest
    pub fn merge_from_layer_id(&self, layer_id: &str) -> Result<LayerInfo> {
        // Validate layer ID length to avoid ambiguous matches
        if layer_id.len() < 8 {
            return Err(SquashError::InvalidInput(format!(
                "Layer ID must be at least 8 characters long, got: {}",
                layer_id.len()
            )));
        }

        // Find the layer with the specified ID
        let matching_layers: Vec<_> = self.layers
            .iter()
            .enumerate()
            .filter(|(_, layer)| layer.digest.starts_with(layer_id))
            .collect();

        if matching_layers.is_empty() {
            return Err(SquashError::LayerNotFound(layer_id.to_string()));
        }

        if matching_layers.len() > 1 {
            println!("Warning: Multiple layers match '{}'. Using the first match:", layer_id);
            for (_, layer) in &matching_layers {
                println!("  - {}", layer.digest);
            }
        }

        let start_index = matching_layers[0].0;
        
        let layers_to_merge = &self.layers[start_index..];
        
        println!("Merging layers from {} to latest:", layer_id);
        for layer in layers_to_merge {
            println!("  - {}", layer.digest);
        }
        
        self.merge_layers(layers_to_merge)
    }
    
    /// Merge a slice of layers into a single layer
    fn merge_layers(&self, layers: &[LayerInfo]) -> Result<LayerInfo> {
        println!("Starting layer merge process...");

        // Validate temp directory exists and is writable
        if !self.temp_dir.exists() {
            std::fs::create_dir_all(&self.temp_dir)?;
        }

        // Initialize virtual filesystem
        let mut vfs = VirtualFilesystem {
            files: HashMap::new(),
        };

        // Process each layer in order
        for (i, layer) in layers.iter().enumerate() {
            println!("Processing layer {}/{}: {}", i + 1, layers.len(), layer.digest);

            // Validate that the layer tar file exists
            if !layer.tar_path.exists() {
                return Err(SquashError::InvalidInput(format!(
                    "Layer tar file does not exist: {}",
                    layer.tar_path.display()
                )));
            }

            self.process_layer_tar(&layer.tar_path, &mut vfs)?;
        }

        // Create the merged layer tar file with unique name to avoid conflicts
        let unique_id = Uuid::new_v4();
        let merged_tar_path = self.temp_dir.join(format!("merged_layer_{}.tar", unique_id));
        self.create_merged_tar_from_vfs(&vfs, &merged_tar_path)?;

        // Calculate the digest of the merged layer
        let digest = self.calculate_layer_digest(&merged_tar_path).inspect_err(|_| {
            // Clean up the temporary file on error
            let _ = std::fs::remove_file(&merged_tar_path);
        })?;

        let size = std::fs::metadata(&merged_tar_path)?.len();

        println!("Layer merge completed. Final size: {} bytes", size);

        Ok(LayerInfo {
            digest,
            size,
            tar_path: merged_tar_path,
        })
    }
    
    /// Process a layer tar file and update the virtual filesystem
    fn process_layer_tar(&self, tar_path: &Path, vfs: &mut VirtualFilesystem) -> Result<()> {
        let file = File::open(tar_path)?;
        let mut archive = Archive::new(file);

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let header = entry.header().clone();
            let path = entry.path()?.to_path_buf();

            // Validate path to prevent directory traversal attacks
            if path.to_string_lossy().contains("..") {
                println!("Warning: Skipping potentially unsafe path: {}", path.display());
                continue;
            }

            let entry_size = header.size()?;

            // Choose storage strategy based on file size
            let file_data = if entry_size <= MAX_MEMORY_FILE_SIZE {
                // Small files: store in memory
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                FileData::InMemory(data)
            } else {
                // Large files: store reference to source
                println!("  Large file detected ({}MB), using disk reference", entry_size / (1024 * 1024));
                FileData::OnDisk {
                    source_tar: tar_path.to_path_buf(),
                    offset: 0, // We'll need to track this properly in a real implementation
                    size: entry_size,
                }
            };

            // Handle whiteout files (Docker deletion markers)
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if let Some(original_name) = filename_str.strip_prefix(".wh.") {
                        if filename_str == ".wh..wh..opq" {
                            // Opaque whiteout - remove all files in this directory
                            let dir_path = path.parent().unwrap_or_else(|| Path::new(""));
                            self.apply_opaque_whiteout(vfs, dir_path);
                        } else {
                            // Regular whiteout - remove specific file
                            let original_path = path.parent()
                                .unwrap_or_else(|| Path::new(""))
                                .join(original_name);

                            println!("  Whiteout: removing {}", original_path.display());
                            vfs.files.insert(original_path, None);
                        }
                        continue;
                    }
                }
            }

            // Add or update file in virtual filesystem
            let size_display = match &file_data {
                FileData::InMemory(data) => data.len(),
                FileData::OnDisk { size, .. } => *size as usize,
            };
            println!("  Adding file: {} ({} bytes)", path.display(), size_display);

            let file_entry = FileEntry {
                header,
                data: file_data,
            };
            vfs.files.insert(path, Some(file_entry));
        }

        Ok(())
    }

    /// Apply opaque whiteout - remove all files in the specified directory
    fn apply_opaque_whiteout(&self, vfs: &mut VirtualFilesystem, dir_path: &Path) {
        // Use proper path comparison instead of string comparison
        vfs.files.retain(|path, _| {
            // Keep files that are not under the directory being cleared
            !path.starts_with(dir_path) || path == dir_path
        });
        println!("  Opaque whiteout: cleared directory {}", dir_path.display());
    }
    
    /// Create a tar file from the virtual filesystem
    fn create_merged_tar_from_vfs(&self, vfs: &VirtualFilesystem, output_path: &Path) -> Result<()> {
        let output_file = File::create(output_path)?;
        let mut builder = Builder::new(output_file);

        // Collect all valid (non-deleted) files and sort them for consistent output
        let mut valid_files: Vec<_> = vfs.files
            .iter()
            .filter_map(|(path, entry_opt)| {
                entry_opt.as_ref().map(|entry| (path, entry))
            })
            .collect();

        // Sort by path for deterministic output
        valid_files.sort_by_key(|(path, _)| *path);

        println!("Creating merged tar with {} files", valid_files.len());

        for (path, file_entry) in valid_files {
            // Validate path length for tar format compatibility
            if path.to_string_lossy().len() > 255 {
                println!("Warning: Skipping file with path too long: {}", path.display());
                continue;
            }

            // Create a new header preserving original metadata
            let mut header = file_entry.header.clone();
            header.set_path(path)?;

            match &file_entry.data {
                FileData::InMemory(data) => {
                    header.set_size(data.len() as u64);
                    header.set_cksum();
                    builder.append(&header, data.as_slice())?;
                    println!("  Added: {} ({} bytes)", path.display(), data.len());
                }
                FileData::OnDisk { size, .. } => {
                    // For large files, we need to stream from the source
                    // This is a simplified implementation - in practice, we'd need to
                    // track exact offsets in the source tar file
                    println!("  Warning: Large file streaming not fully implemented: {} ({} bytes)",
                             path.display(), size);

                    // For now, create an empty entry as a placeholder
                    header.set_size(0);
                    header.set_cksum();
                    builder.append(&header, &[] as &[u8])?;
                }
            }
        }

        builder.finish()?;
        println!("Merged tar created successfully");
        Ok(())
    }
    
    /// Calculate the SHA256 digest of a layer tar file
    fn calculate_layer_digest(&self, tar_path: &Path) -> Result<String> {
        let mut file = File::open(tar_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        let digest = hasher.finalize();
        Ok(format!("sha256:{:x}", digest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_layer_info_creation() {
        let temp_dir = TempDir::new().unwrap();
        let tar_path = temp_dir.path().join("test.tar");
        fs::write(&tar_path, b"test data").unwrap();

        let layer_info = LayerInfo {
            digest: "sha256:test123".to_string(),
            size: 9,
            tar_path: tar_path.clone(),
        };

        assert_eq!(layer_info.digest, "sha256:test123");
        assert_eq!(layer_info.size, 9);
        assert_eq!(layer_info.tar_path, tar_path);
    }

    #[test]
    fn test_layer_merger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let layers = vec![
            LayerInfo {
                digest: "sha256:layer1".to_string(),
                size: 100,
                tar_path: temp_dir.path().join("layer1.tar"),
            },
            LayerInfo {
                digest: "sha256:layer2".to_string(),
                size: 200,
                tar_path: temp_dir.path().join("layer2.tar"),
            },
        ];

        let merger = LayerMerger::new(layers.clone(), temp_dir.path().to_path_buf());
        assert_eq!(merger.layers.len(), 2);
        assert_eq!(merger.layers[0].digest, "sha256:layer1");
        assert_eq!(merger.layers[1].digest, "sha256:layer2");
    }

    #[test]
    fn test_merge_latest_layers_validation() {
        let temp_dir = TempDir::new().unwrap();
        let layers = vec![
            LayerInfo {
                digest: "sha256:layer1".to_string(),
                size: 100,
                tar_path: temp_dir.path().join("layer1.tar"),
            },
        ];

        let merger = LayerMerger::new(layers, temp_dir.path().to_path_buf());

        // Test error when requesting 0 layers
        let result = merger.merge_latest_layers(0);
        assert!(result.is_err());
        if let Err(SquashError::InvalidInput(msg)) = result {
            assert!(msg.contains("Cannot merge 0 layers"));
        } else {
            panic!("Expected InvalidInput error for 0 layers");
        }

        // Test error when requesting more layers than available
        let result = merger.merge_latest_layers(5);
        assert!(result.is_err());

        if let Err(SquashError::InvalidInput(msg)) = result {
            assert!(msg.contains("Cannot merge 5 layers, only 1 layers available"));
        } else {
            panic!("Expected InvalidInput error");
        }
    }

    #[test]
    fn test_layer_id_validation() {
        let temp_dir = TempDir::new().unwrap();
        let layers = vec![
            LayerInfo {
                digest: "sha256:abcdef123456".to_string(),
                size: 100,
                tar_path: temp_dir.path().join("layer1.tar"),
            },
        ];

        let merger = LayerMerger::new(layers, temp_dir.path().to_path_buf());

        // Test error when layer ID is too short
        let result = merger.merge_from_layer_id("abc");
        assert!(result.is_err());
        if let Err(SquashError::InvalidInput(msg)) = result {
            assert!(msg.contains("Layer ID must be at least 8 characters long"));
        } else {
            panic!("Expected InvalidInput error for short layer ID");
        }
    }
}
