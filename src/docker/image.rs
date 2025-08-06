use crate::error::{Result, SquashError};
use crate::docker::{TarExtractor, LayerMerger, LayerInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Docker image manifest structure as found in manifest.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockerManifest {
    /// Path to the config file (usually config.json)
    #[serde(rename = "Config")]
    pub config: String,
    /// Repository tags for this image
    #[serde(rename = "RepoTags")]
    pub repo_tags: Option<Vec<String>>,
    /// List of layer tar files
    #[serde(rename = "Layers")]
    pub layers: Vec<String>,
}

/// Docker image configuration structure as found in config.json
#[derive(Debug, Deserialize, Serialize)]
pub struct DockerConfig {
    /// Target architecture (e.g., "amd64")
    pub architecture: String,
    /// Container configuration details
    pub config: ConfigDetails,
    /// Root filesystem information
    pub rootfs: RootFs,
    /// Layer history information
    pub history: Vec<HistoryEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigDetails {
    #[serde(rename = "Env")]
    pub env: Option<Vec<String>>,
    #[serde(rename = "Cmd")]
    pub cmd: Option<Vec<String>>,
    #[serde(rename = "WorkingDir")]
    pub working_dir: Option<String>,
    #[serde(rename = "ExposedPorts")]
    pub exposed_ports: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RootFs {
    #[serde(rename = "type")]
    pub fs_type: String,
    pub diff_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HistoryEntry {
    pub created: String,
    pub created_by: String,
    pub empty_layer: Option<bool>,
}

pub struct DockerImage {
    pub manifest: DockerManifest,
    pub config: DockerConfig,
    pub source_path: PathBuf,
    pub layers: Vec<LayerInfo>,
    pub temp_dir: Option<TempDir>,
}

impl DockerImage {
    /// Load a Docker image from a file or export from Docker
    pub fn load(source: &str, temp_dir: Option<&Path>) -> Result<Self> {
        let source_path = if source.contains(':') && !Path::new(source).exists() {
            // Assume it's an image name:tag, export it first
            Self::export_image(source, temp_dir)?
        } else {
            // Assume it's a file path
            PathBuf::from(source)
        };

        if !source_path.exists() {
            return Err(SquashError::InvalidInput(format!(
                "Source file does not exist: {}",
                source_path.display()
            )));
        }

        // Extract and parse the image
        let (manifest, config, layers, temp_dir) = Self::parse_image(&source_path)?;

        Ok(DockerImage {
            manifest,
            config,
            source_path,
            layers,
            temp_dir: Some(temp_dir),
        })
    }

    /// Export a Docker image using docker save
    fn export_image(image_name: &str, temp_dir: Option<&Path>) -> Result<PathBuf> {
        let temp_dir = temp_dir.unwrap_or_else(|| Path::new("/tmp"));
        let output_path = temp_dir.join(format!("{}.tar", image_name.replace(':', "_")));

        let output = Command::new("docker")
            .args(["save", "-o", output_path.to_str().unwrap(), image_name])
            .output()
            .map_err(|e| SquashError::DockerError(format!("Failed to run docker save: {}", e)))?;

        if !output.status.success() {
            return Err(SquashError::DockerError(format!(
                "docker save failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(output_path)
    }

    /// Parse manifest and config from Docker image tar
    fn parse_image(image_path: &Path) -> Result<(DockerManifest, DockerConfig, Vec<LayerInfo>, TempDir)> {
        println!("Extracting Docker image: {}", image_path.display());

        // Extract the Docker image tar file
        let extractor = TarExtractor::extract(image_path)?;

        // Read and parse manifest.json
        if !extractor.file_exists("manifest.json") {
            return Err(SquashError::InvalidInput(
                "manifest.json not found in Docker image".to_string()
            ));
        }

        let manifest_content = extractor.read_file("manifest.json")?;
        let manifests: Vec<DockerManifest> = serde_json::from_str(&manifest_content)?;

        if manifests.is_empty() {
            return Err(SquashError::InvalidInput(
                "No manifests found in manifest.json".to_string()
            ));
        }

        let manifest = manifests[0].clone();

        // Read and parse the config file
        let config_content = extractor.read_file(&manifest.config)?;
        let config: DockerConfig = serde_json::from_str(&config_content)?;

        // Create layer info from manifest layers
        let mut layers = Vec::new();
        for (i, layer_path) in manifest.layers.iter().enumerate() {
            let layer_tar_path = extractor.get_file_path(layer_path);

            if !layer_tar_path.exists() {
                return Err(SquashError::InvalidInput(format!(
                    "Layer file not found: {}", layer_path
                )));
            }

            // Use diff_id from config if available, otherwise generate from layer path
            let digest = if i < config.rootfs.diff_ids.len() {
                config.rootfs.diff_ids[i].clone()
            } else {
                format!("sha256:{}", layer_path.replace(".tar", "").replace("/", ""))
            };

            let size = std::fs::metadata(&layer_tar_path)?.len();

            layers.push(LayerInfo {
                digest,
                size,
                tar_path: layer_tar_path,
            });
        }

        println!("Parsed {} layers from Docker image", layers.len());

        Ok((manifest, config, layers, extractor.temp_dir))
    }

    /// Squash layers according to the specification
    pub fn squash_layers(&mut self, layer_spec: &str) -> Result<()> {
        if self.layers.is_empty() {
            return Err(SquashError::InvalidInput("No layers to merge".to_string()));
        }

        // Create a temporary directory for the merge operation
        let temp_dir = self.temp_dir.as_ref()
            .ok_or_else(|| SquashError::InvalidInput("No temp directory available".to_string()))?
            .path().to_path_buf();

        let merger = LayerMerger::new(self.layers.clone(), temp_dir);

        // Parse layer specification and merge layers
        let merged_layer = if let Ok(count) = layer_spec.parse::<usize>() {
            // Merge latest n layers
            if count > self.layers.len() {
                return Err(SquashError::InvalidInput(format!(
                    "Cannot merge {} layers, image only has {} layers",
                    count,
                    self.layers.len()
                )));
            }
            merger.merge_latest_layers(count)?
        } else {
            // Find layer by ID and merge from that layer to latest
            merger.merge_from_layer_id(layer_spec)?
        };

        // Update the image with the merged layer
        let layers_to_merge_count = if let Ok(count) = layer_spec.parse::<usize>() {
            count
        } else {
            // Find the layer and count from there
            let start_index = self.layers
                .iter()
                .position(|layer| layer.digest.starts_with(layer_spec))
                .ok_or_else(|| SquashError::LayerNotFound(layer_spec.to_string()))?;
            self.layers.len() - start_index
        };

        // Remove the merged layers and add the new merged layer
        self.layers.truncate(self.layers.len() - layers_to_merge_count);
        self.layers.push(merged_layer);

        // Update manifest layers
        let remaining_layers = self.manifest.layers.len() - layers_to_merge_count;
        self.manifest.layers.truncate(remaining_layers);
        self.manifest.layers.push("merged_layer.tar".to_string());

        // Update config diff_ids
        self.config.rootfs.diff_ids.truncate(remaining_layers);
        self.config.rootfs.diff_ids.push(self.layers.last().unwrap().digest.clone());

        println!("Successfully merged layers. New layer count: {}", self.layers.len());

        Ok(())
    }

    /// Save the squashed image to a file
    pub fn save_to_file(&self, output_path: &Path) -> Result<()> {
        use crate::docker::TarBuilder;

        println!("Saving squashed image to: {}", output_path.display());

        // Create a new tar builder
        let builder = TarBuilder::new()?;

        // Add the updated manifest.json
        let manifest_json = serde_json::to_string_pretty(&vec![&self.manifest])?;
        builder.add_file("manifest.json", manifest_json.as_bytes())?;

        // Add the updated config file
        let config_json = serde_json::to_string_pretty(&self.config)?;
        builder.add_file(&self.manifest.config, config_json.as_bytes())?;

        // Add all layer files
        for (i, layer) in self.layers.iter().enumerate() {
            let layer_filename = if i == self.layers.len() - 1 {
                // This is the merged layer
                "merged_layer.tar"
            } else {
                &self.manifest.layers[i]
            };

            // Copy the layer tar file
            let layer_content = std::fs::read(&layer.tar_path)?;
            builder.add_file(layer_filename, &layer_content)?;
        }

        // Build the final tar file
        builder.build(output_path)?;

        println!("Successfully saved squashed image to: {}", output_path.display());
        Ok(())
    }

    /// Load the squashed image into Docker
    pub fn load_into_docker(&self, image_name: &str) -> Result<()> {
        // First save to a temporary file
        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path();

        self.save_to_file(temp_path)?;

        println!("Loading squashed image into Docker as: {}", image_name);

        // Use docker load to import the image
        let output = Command::new("docker")
            .args(["load", "-i", temp_path.to_str().unwrap()])
            .output()
            .map_err(|e| SquashError::DockerError(format!("Failed to run docker load: {}", e)))?;

        if !output.status.success() {
            return Err(SquashError::DockerError(format!(
                "docker load failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Tag the image with the specified name
        if let Some(original_tag) = self.manifest.repo_tags.as_ref().and_then(|tags| tags.first()) {
            let tag_output = Command::new("docker")
                .args(["tag", original_tag, image_name])
                .output()
                .map_err(|e| SquashError::DockerError(format!("Failed to run docker tag: {}", e)))?;

            if !tag_output.status.success() {
                return Err(SquashError::DockerError(format!(
                    "docker tag failed: {}",
                    String::from_utf8_lossy(&tag_output.stderr)
                )));
            }
        }

        println!("Successfully loaded squashed image into Docker as: {}", image_name);
        Ok(())
    }
}
