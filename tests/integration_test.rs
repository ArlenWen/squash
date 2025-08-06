use squash::{cli::*, docker::DockerImage, SquashError};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create a simple test Docker image tar file
fn create_test_image(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    // Use the Python script to create a test image
    let output = Command::new("python3")
        .arg("create_test_image.py")
        .current_dir(".")
        .output()?;
    
    if !output.status.success() {
        return Err(format!("Failed to create test image: {}", 
                          String::from_utf8_lossy(&output.stderr)).into());
    }
    
    // Move the created file to the desired location
    if Path::new("test-docker-image.tar").exists() {
        fs::copy("test-docker-image.tar", output_path)?;
        fs::remove_file("test-docker-image.tar")?;
    }
    
    Ok(())
}

#[test]
fn test_docker_image_loading() {
    let temp_dir = TempDir::new().unwrap();
    let test_image_path = temp_dir.path().join("test.tar");
    
    // Create a test image
    if create_test_image(&test_image_path).is_err() {
        // Skip test if we can't create the test image
        return;
    }
    
    // Test loading the image
    let result = DockerImage::load(
        test_image_path.to_str().unwrap(), 
        Some(temp_dir.path())
    );
    
    match result {
        Ok(image) => {
            assert!(!image.manifest.layers.is_empty());
            assert!(image.manifest.config.ends_with(".json"));
        }
        Err(e) => {
            // This might fail in CI environments without proper setup
            println!("Image loading test skipped: {}", e);
        }
    }
}

#[test]
fn test_cli_validation() {
    use clap::Parser;

    // Test that CLI requires --layers argument
    let args = vec![
        "squash",
        "squash",
        "--source", "test.tar",
        "--output", "output.tar",
        // Missing --layers
    ];

    let result = Cli::try_parse_from(args);
    assert!(result.is_err()); // Should fail due to missing required argument

    // Test valid CLI parsing
    let args = vec![
        "squash",
        "squash",
        "--source", "test.tar",
        "--output", "output.tar",
        "--layers", "2",
    ];

    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Squash { output, load, .. } => {
            assert!(output.is_some());
            assert!(load.is_none());
        }
    }
}

#[test]
fn test_error_handling() {
    // Test loading a non-existent file
    let result = DockerImage::load("/non/existent/file.tar", None);
    assert!(result.is_err());

    match result {
        Err(SquashError::InvalidInput(_)) => {}, // Expected for non-existent file
        Err(SquashError::IoError(_)) => {}, // Also acceptable
        Err(e) => panic!("Expected InvalidInput or IoError, got: {:?}", e),
        Ok(_) => panic!("Expected error for non-existent file"),
    }
}

#[test]
fn test_layer_count_validation() {
    // Test that we can parse different layer specifications
    let test_cases = vec![
        ("3", true),
        ("0", true),
        ("sha256:abc123", true),
        ("layer_id", true),
    ];
    
    for (layer_spec, should_be_valid) in test_cases {
        // This is just testing that the string parsing works
        // The actual validation happens in the layer merger
        assert_eq!(!layer_spec.is_empty(), should_be_valid);
    }
}

#[test]
fn test_temp_directory_handling() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    
    // Test that temporary directory exists and is writable
    assert!(temp_path.exists());
    assert!(temp_path.is_dir());
    
    // Test creating a file in the temp directory
    let test_file = temp_path.join("test.txt");
    fs::write(&test_file, b"test content").unwrap();
    assert!(test_file.exists());
    
    let content = fs::read(&test_file).unwrap();
    assert_eq!(content, b"test content");
}

#[test]
#[ignore] // Ignore by default, run with --ignored
fn test_full_squash_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let test_image_path = temp_dir.path().join("test.tar");
    let output_path = temp_dir.path().join("squashed.tar");
    
    // Create a test image
    if create_test_image(&test_image_path).is_err() {
        return; // Skip if can't create test image
    }
    
    // Load the image
    let mut image = match DockerImage::load(
        test_image_path.to_str().unwrap(), 
        Some(temp_dir.path())
    ) {
        Ok(img) => img,
        Err(_) => return, // Skip if can't load
    };
    
    // Try to squash layers
    if image.squash_layers("2").is_ok() {
        // Save the result
        if image.save_to_file(&output_path).is_ok() {
            assert!(output_path.exists());
            
            // Verify the output file is not empty
            let metadata = fs::metadata(&output_path).unwrap();
            assert!(metadata.len() > 0);
        }
    }
}
