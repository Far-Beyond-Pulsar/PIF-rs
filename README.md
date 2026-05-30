# Pulsar Image Format (PIF)

A Git-friendly, high-performance image format designed for real-time texture editing in game engines.

## Features

- **🎨 Hybrid Raster/Vector**: Mix raster painting with vector shapes in a single document
- **📦 Dual Format**: Directory bundles for development, ZIP archives for distribution
- **⚡ Fast Encoding**: QOI compression for linear-time tile encoding/decoding
- **💾 Sparse Storage**: Unpainted regions occupy zero disk space
- **🔒 Atomic Saves**: Transactional commits prevent corruption
- **🌳 Git-Friendly**: JSON manifest + binary shards = clean diffs

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
pulsar_image_format = { git = "https://github.com/Far-Beyond-Pulsar/PIF-rs" }
```

### Create a New PIF Asset

```rust
use pulsar_image_format::{PifAssetManager, SaveMode, model::Layer};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a 1024×768 canvas
    let mut manager = PifAssetManager::create_new(
        "my_sprite.pif",
        1024,
        768,
        SaveMode::Directory
    )?;

    // Add a raster layer
    manager.manifest_mut().layers.push(Layer::Raster {
        id: "background".to_string(),
        name: "Background".to_string(),
        visible: true,
        opacity: 1.0,
        blend_mode: "normal".to_string(),
        tile_size: 256,
        tiles: HashMap::new(),
    });

    // Paint a red pixel at tile (0, 0)
    let mut pixels = manager.load_raster_tile("background", 0, 0)?;
    pixels[0..4].copy_from_slice(&[255, 0, 0, 255]);

    // Commit atomically
    let mut changes = HashMap::new();
    changes.insert(("background".to_string(), 0, 0), pixels);
    manager.commit_changes(changes)?;

    Ok(())
}
```

### Open and Modify

```rust
use pulsar_image_format::PifAssetManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Automatically detects directory or archive format
    let mut manager = PifAssetManager::open("my_sprite.pif")?;

    println!("Canvas: {}×{}", 
        manager.manifest().canvas.width,
        manager.manifest().canvas.height
    );

    // Modify layer visibility
    if let Some(layer) = manager.manifest_mut().layers.first_mut() {
        match layer {
            pulsar_image_format::model::Layer::Raster { visible, .. } => {
                *visible = false;
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Export to Standard Formats

```rust
use pulsar_image_format::PifAssetManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = PifAssetManager::open("logo.pif")?;
    
    // Export vector layers to SVG
    manager.bake("output.svg", "svg")?;
    
    Ok(())
}
```

## Format Structure

### Directory Bundle (Development Mode)

```
my_image.pif/
├── manifest.json          # Human-readable layer metadata
└── .raster/               # Binary tile data (QOI compressed)
    ├── layer_bg_0_0.qoi
    ├── layer_bg_0_1.qoi
    └── layer_fg_2_3.qoi
```

**manifest.json** example:

```json
{
  "pif_version": "1.0",
  "canvas": {
    "width": 1024,
    "height": 768,
    "color_space": "sRGB"
  },
  "layers": [
    {
      "type": "raster",
      "id": "bg",
      "name": "Background",
      "visible": true,
      "opacity": 1.0,
      "blend_mode": "normal",
      "tile_size": 256,
      "tiles": {
        "0_0": ".raster/layer_bg_0_0.qoi",
        "0_1": ".raster/layer_bg_0_1.qoi"
      }
    }
  ]
}
```

### Archive Format (Distribution Mode)

A single `.pif` ZIP file containing the same structure:

```
my_image.pif               # ZIP archive
├─ manifest.json
├─ .raster/layer_bg_0_0.qoi
└─ .raster/layer_bg_0_1.qoi
```

## Sparse Tiling

The canvas is divided into 256×256 pixel tiles. **Unpainted tiles are not stored**, making the format extremely space-efficient for large canvases with localized edits.

```
┌───────┬───────┬───────┬───────┐
│ (0,0) │ (1,0) │ (2,0) │ (3,0) │  ← Only tiles with painted
├───────┼───────┼───────┼───────┤    pixels exist on disk
│ (0,1) │ █████ │       │       │
├───────┼─█████ ┼───────┼───────┤  █ = Stored tile
│ (0,2) │ █████ │ (2,2) │       │    (others return transparent)
├───────┼───────┼───────┼───────┤
│ (0,3) │ (1,3) │ (2,3) │ (3,3) │
└───────┴───────┴───────┴───────┘
```

## Why QOI?

Traditional PNG compression creates micro-stutters during auto-saves in real-time editors. **QOI (Quite OK Image)** provides:

- Lossless compression comparable to PNG
- **4× faster encoding** (linear streaming)
- **3× faster decoding**
- Zero external dependencies

Perfect for interactive tools where frame drops are unacceptable.

## API Reference

### Core Types

- **`PifAssetManager`** - Main API for creating, opening, and editing PIF assets
- **`SaveMode::Directory`** - Git-friendly directory bundle
- **`SaveMode::Archive`** - Single-file ZIP distribution format
- **`Layer::Raster`** - Pixel-based layer with sparse tiling
- **`Layer::Vector`** - SVG-compatible vector shapes

### Key Methods

- `create_new(path, width, height, mode)` - Initialize new asset
- `open(path)` - Open existing asset (auto-detects format)
- `load_raster_tile(layer_id, x, y)` - Fetch 256×256 pixel buffer
- `commit_changes(dirty_tiles)` - Atomic transactional save
- `bake(output_path, format)` - Export to PNG/SVG

## Performance Characteristics

- **Tile Load**: ~0.2ms per tile (QOI decode)
- **Tile Save**: ~0.4ms per tile (QOI encode + disk write)
- **Sparse Lookup**: O(1) HashMap check
- **Memory Footprint**: 256KB per active tile (uncompressed RGBA)

## Design Rationale

### Why Not Just Use PNG Layers?

Individual PNG files per tile would create thousands of small files, overwhelming Git and filesystems. The manifest provides a single source of truth for tile organization.

### Why Not a Monolithic Binary Format?

Git cannot diff binary blobs effectively. The PIF's hybrid approach gives clean diffs for structural changes (layer order, visibility, vector shapes) while keeping pixel data opaque but sharded.

### Why Two Save Modes?

- **Directory bundles** enable granular Git tracking and partial tile loading
- **Archives** simplify distribution and reduce file handle exhaustion

## License

See the main Pulsar Engine license.

## Contributing

This crate is part of the Pulsar Engine project. See `CONTRIBUTING.md` in the repository root.
