use criterion::{black_box, criterion_group, criterion_main, Criterion};
use squash::docker::{LayerInfo, LayerMerger};
use std::fs;
use tempfile::TempDir;

fn create_test_layer(temp_dir: &TempDir, name: &str, size: usize) -> LayerInfo {
    let tar_path = temp_dir.path().join(format!("{}.tar", name));
    
    // Create a dummy tar file with specified size
    let dummy_data = vec![0u8; size];
    fs::write(&tar_path, dummy_data).unwrap();
    
    LayerInfo {
        digest: format!("sha256:{}", name),
        size: size as u64,
        tar_path,
    }
}

fn benchmark_layer_merger_creation(c: &mut Criterion) {
    c.bench_function("layer_merger_creation", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let layers = vec![
                create_test_layer(&temp_dir, "layer1", 1024),
                create_test_layer(&temp_dir, "layer2", 2048),
                create_test_layer(&temp_dir, "layer3", 4096),
            ];
            
            let merger = LayerMerger::new(
                black_box(layers), 
                black_box(temp_dir.path().to_path_buf())
            );
            
            black_box(merger)
        })
    });
}

fn benchmark_layer_info_creation(c: &mut Criterion) {
    c.bench_function("layer_info_creation", |b| {
        let temp_dir = TempDir::new().unwrap();
        let tar_path = temp_dir.path().join("test.tar");
        fs::write(&tar_path, b"test data").unwrap();
        
        b.iter(|| {
            let layer_info = LayerInfo {
                digest: black_box("sha256:test123".to_string()),
                size: black_box(9),
                tar_path: black_box(tar_path.clone()),
            };
            
            black_box(layer_info)
        })
    });
}

fn benchmark_multiple_layers(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_layers");
    
    for layer_count in [5, 10, 20].iter() {
        group.bench_with_input(
            format!("create_{}_layers", layer_count),
            layer_count,
            |b, &layer_count| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let mut layers = Vec::new();
                    
                    for i in 0..layer_count {
                        layers.push(create_test_layer(
                            &temp_dir, 
                            &format!("layer{}", i), 
                            1024 * (i + 1)
                        ));
                    }
                    
                    let merger = LayerMerger::new(
                        black_box(layers), 
                        black_box(temp_dir.path().to_path_buf())
                    );
                    
                    black_box(merger)
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_large_layers(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_layers");
    
    for size_kb in [1, 10, 100].iter() {
        group.bench_with_input(
            format!("layer_{}kb", size_kb),
            size_kb,
            |b, &size_kb| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let layer = create_test_layer(
                        &temp_dir, 
                        "large_layer", 
                        black_box(size_kb * 1024)
                    );
                    
                    black_box(layer)
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_layer_merger_creation,
    benchmark_layer_info_creation,
    benchmark_multiple_layers,
    benchmark_large_layers
);
criterion_main!(benches);
