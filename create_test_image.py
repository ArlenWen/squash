#!/usr/bin/env python3
"""
Create a test Docker image tar file for testing the squash tool.
This creates a minimal Docker image structure with manifest.json and config files.
"""

import json
import os
import tarfile
import tempfile
import shutil

def create_test_docker_image(output_path):
    """Create a test Docker image tar file"""
    
    with tempfile.TemporaryDirectory() as temp_dir:
        # Create manifest.json
        manifest = [{
            "Config": "config.json",
            "RepoTags": ["test:latest"],
            "Layers": [
                "layer1.tar",
                "layer2.tar",
                "layer3.tar"
            ]
        }]
        
        with open(os.path.join(temp_dir, "manifest.json"), "w") as f:
            json.dump(manifest, f, indent=2)
        
        # Create config.json
        config = {
            "architecture": "amd64",
            "config": {
                "Env": ["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],
                "Cmd": ["/bin/sh"],
                "WorkingDir": "/",
                "ExposedPorts": None
            },
            "rootfs": {
                "type": "layers",
                "diff_ids": [
                    "sha256:layer1digest123456789abcdef",
                    "sha256:layer2digest123456789abcdef", 
                    "sha256:layer3digest123456789abcdef"
                ]
            },
            "history": [
                {
                    "created": "2024-01-01T00:00:00Z",
                    "created_by": "test layer 1",
                    "empty_layer": False
                },
                {
                    "created": "2024-01-01T00:01:00Z", 
                    "created_by": "test layer 2",
                    "empty_layer": False
                },
                {
                    "created": "2024-01-01T00:02:00Z",
                    "created_by": "test layer 3", 
                    "empty_layer": False
                }
            ]
        }
        
        with open(os.path.join(temp_dir, "config.json"), "w") as f:
            json.dump(config, f, indent=2)
        
        # Create test layer tar files
        for i, layer_name in enumerate(["layer1.tar", "layer2.tar", "layer3.tar"]):
            layer_path = os.path.join(temp_dir, layer_name)
            
            # Create a temporary directory for layer content
            with tempfile.TemporaryDirectory() as layer_temp:
                # Create some test files for this layer
                test_file = os.path.join(layer_temp, f"test_file_{i+1}.txt")
                with open(test_file, "w") as f:
                    f.write(f"This is test file from layer {i+1}\n")
                
                # Create a subdirectory with a file
                subdir = os.path.join(layer_temp, f"subdir_{i+1}")
                os.makedirs(subdir)
                subfile = os.path.join(subdir, "subfile.txt")
                with open(subfile, "w") as f:
                    f.write(f"This is a subfile from layer {i+1}\n")
                
                # Create the layer tar file
                with tarfile.open(layer_path, "w") as layer_tar:
                    layer_tar.add(test_file, arcname=f"test_file_{i+1}.txt")
                    layer_tar.add(subdir, arcname=f"subdir_{i+1}")
        
        # Create the final Docker image tar file
        with tarfile.open(output_path, "w") as docker_tar:
            docker_tar.add(os.path.join(temp_dir, "manifest.json"), arcname="manifest.json")
            docker_tar.add(os.path.join(temp_dir, "config.json"), arcname="config.json")
            docker_tar.add(os.path.join(temp_dir, "layer1.tar"), arcname="layer1.tar")
            docker_tar.add(os.path.join(temp_dir, "layer2.tar"), arcname="layer2.tar")
            docker_tar.add(os.path.join(temp_dir, "layer3.tar"), arcname="layer3.tar")
    
    print(f"Created test Docker image: {output_path}")

if __name__ == "__main__":
    create_test_docker_image("test-docker-image.tar")
