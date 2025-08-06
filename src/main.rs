use squash::{cli::*, docker::DockerImage, SquashError};
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), SquashError> {
    let cli = Cli::parse_args();

    match cli.command {
        Commands::Squash {
            source,
            output,
            load,
            temp_dir,
            layers,
            verbose,
        } => {
            if verbose {
                println!("Loading Docker image from: {}", source);
            }

            // Validate arguments
            if output.is_none() && load.is_none() {
                return Err(SquashError::InvalidInput(
                    "Either --output or --load must be specified".to_string(),
                ));
            }

            // Load the Docker image
            let mut image = DockerImage::load(&source, temp_dir.as_deref())?;

            if verbose {
                println!("Image loaded successfully");
                println!("Layers: {}", image.manifest.layers.len());
                if let Some(tags) = &image.manifest.repo_tags {
                    println!("Tags: {:?}", tags);
                }
            }

            // Squash the layers
            if verbose {
                println!("Squashing layers: {}", layers);
            }
            image.squash_layers(&layers)?;

            // Output the result
            if let Some(output_path) = output {
                if verbose {
                    println!("Saving to file: {}", output_path.display());
                }
                image.save_to_file(&output_path)?;
            }

            if let Some(image_name) = load {
                if verbose {
                    println!("Loading into Docker as: {}", image_name);
                }
                image.load_into_docker(&image_name)?;
            }

            println!("Image squashing completed successfully!");
        }
    }

    Ok(())
}
