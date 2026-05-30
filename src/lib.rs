//! # Pulsar Image Format (.pif)
//!
//! A hybrid text/binary directory structure built for real-time engine texture streaming
//! and elegant version control integration.
//!
//! ## Features
//!
//! - **Git-Friendly**: Text-based JSON manifest with sparse binary tiles
//! - **High Performance**: QOI-compressed tiles for lightning-fast encoding/decoding
//! - **Sparse Storage**: Unpainted regions consume zero disk space
//! - **Atomic Commits**: Transactional saves prevent corruption
//! - **Dual Format**: Directory bundles for development, ZIP archives for distribution
//! - **Vector Support**: Mix raster and vector layers in a single document
//!
//! ## Architecture
//!
//! The PIF format uses a **sparse tiling grid** where canvas layers are divided into
//! fixed 256×256 pixel chunks. If a region hasn't been painted on, it occupies zero
//! bytes on disk with no representation in the file hierarchy.
//!
//! ### Directory Bundle Structure
//!
//! ```text
//! my_image.pif/
//! ├── manifest.json          # Human-readable metadata
//! └── .raster/               # Binary pixel data
//!     ├── layer_bg_0_0.qoi
//!     ├── layer_bg_0_1.qoi
//!     └── layer_fg_1_0.qoi
//! ```
//!
//! ### Archive Structure
//!
//! ```text
//! my_image.pif               # Single ZIP file
//! ├─ manifest.json
//! ├─ .raster/layer_bg_0_0.qoi
//! ├─ .raster/layer_bg_0_1.qoi
//! └─ .raster/layer_fg_1_0.qoi
//! ```
//!
//! ## Example Usage
//!
//! ### Creating and Editing a PIF Asset
//!
//! ```rust
//! use pulsar_image_format::{PifAssetManager, SaveMode};
//! use pulsar_image_format::model::Layer;
//! use std::collections::HashMap;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let temp_dir = tempfile::tempdir()?;
//! # let path = temp_dir.path().join("hero_sprite.pif");
//! // Create a new 1024x1024 PIF asset as a directory bundle
//! let mut manager = PifAssetManager::create_new(
//!     &path,
//!     1024,
//!     1024,
//!     SaveMode::Directory
//! )?;
//!
//! // Add a raster layer
//! manager.manifest_mut().layers.push(Layer::Raster {
//!     id: "character".to_string(),
//!     name: "Character".to_string(),
//!     visible: true,
//!     opacity: 1.0,
//!     blend_mode: "normal".to_string(),
//!     tile_size: 256,
//!     tiles: Default::default(),
//! });
//!
//! // Load a tile (returns transparent buffer if unpainted)
//! let tile_pixels = manager.load_raster_tile("character", 0, 0)?;
//!
//! // Paint some pixels
//! let mut edited = tile_pixels.clone();
//! edited[0..4].copy_from_slice(&[255, 0, 85, 255]); // Hot pink!
//!
//! // Commit changes atomically
//! let mut changes = HashMap::new();
//! changes.insert(("character".to_string(), 0, 0), edited);
//! manager.commit_changes(changes)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Opening an Existing Asset
//!
//! ```rust
//! use pulsar_image_format::PifAssetManager;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Automatically detects directory or archive format
//! let manager = PifAssetManager::open("assets/player.pif")?;
//!
//! println!("Canvas: {}×{}", 
//!     manager.manifest().canvas.width,
//!     manager.manifest().canvas.height
//! );
//!
//! for layer in &manager.manifest().layers {
//!     println!("Layer: {}", layer.name());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Exporting to Standard Formats
//!
//! ```rust,no_run
//! use pulsar_image_format::PifAssetManager;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = PifAssetManager::open("art/logo.pif")?;
//!
//! // Export vector layers to SVG
//! manager.bake("output/logo.svg", "svg")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Module Organization
//!
//! - [`model`] - Data structures (manifest, layers, canvas config)
//! - [`manager`] - High-level API via `PifAssetManager`
//! - [`tile`] - Tile operations (encoding, decoding, coordinate math)
//! - [`directory`] - Directory bundle I/O operations
//! - [`archive`] - ZIP archive I/O operations
//! - [`bake`] - Export to PNG, SVG, and other formats
//! - [`error`] - Error types and result aliases

pub mod model;
pub mod error;
pub mod tile;
pub mod directory;
pub mod archive;
pub mod bake;
pub mod manager;

// Re-export main types for convenience
pub use error::{PifError, Result};
pub use manager::{PifAssetManager, SaveMode};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use model::Layer;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_full_workflow_directory() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");

        // Create
        let mut manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory).unwrap();

        // Add layer
        manager.manifest_mut().layers.push(Layer::Raster {
            id: "bg".to_string(),
            name: "Background".to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            tile_size: 256,
            tiles: HashMap::new(),
        });

        // Edit
        let mut pixels = vec![0u8; 256 * 256 * 4];
        pixels[0] = 128;
        let mut changes = HashMap::new();
        changes.insert(("bg".to_string(), 0, 0), pixels);
        manager.commit_changes(changes).unwrap();

        // Reopen and verify
        let manager2 = PifAssetManager::open(&path).unwrap();
        let loaded = manager2.load_raster_tile("bg", 0, 0).unwrap();
        assert_eq!(loaded[0], 128);
    }

    #[test]
    fn test_full_workflow_archive() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");

        // Create archive
        let mut manager = PifAssetManager::create_new(&path, 256, 256, SaveMode::Archive).unwrap();

        // Add layer
        manager.manifest_mut().layers.push(Layer::Raster {
            id: "layer1".to_string(),
            name: "Layer 1".to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            tile_size: 256,
            tiles: HashMap::new(),
        });

        // Edit
        let pixels = vec![255u8; 256 * 256 * 4];
        let mut changes = HashMap::new();
        changes.insert(("layer1".to_string(), 0, 0), pixels);
        manager.commit_changes(changes).unwrap();

        // Reopen
        let manager2 = PifAssetManager::open(&path).unwrap();
        assert_eq!(manager2.save_mode(), SaveMode::Archive);
        
        let loaded = manager2.load_raster_tile("layer1", 0, 0).unwrap();
        assert!(loaded.iter().all(|&b| b == 255));
    }
}
