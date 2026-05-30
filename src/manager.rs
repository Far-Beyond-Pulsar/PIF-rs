//! PIF Asset Manager - High-level API for working with PIF assets
//!
//! This module provides the `PifAssetManager` struct, which is the main entry point
//! for creating, opening, and manipulating PIF assets in both directory and archive formats.

use crate::archive;
use crate::bake;
use crate::directory;
use crate::error::{PifError, Result};
use crate::model::{Layer, PifManifest};
use crate::tile;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Save mode for PIF assets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveMode {
    /// Save as a directory bundle (Git-friendly, development mode)
    ///
    /// In this mode, the manifest is stored as `manifest.json` and tiles are
    /// stored as individual QOI files in the `.raster/` subdirectory.
    Directory,
    
    /// Save as a single ZIP archive file (distribution mode)
    ///
    /// In this mode, everything is packaged into a single `.pif` file using
    /// ZIP compression for easier distribution.
    Archive,
}

/// Manager for PIF assets, handling both directory bundles and single-file archives
///
/// This is the main entry point for working with PIF files. It provides methods for
/// creating, opening, and manipulating PIF assets in both directory and archive formats.
///
/// # Examples
///
/// ```rust
/// use pulsar_image_format::{PifAssetManager, SaveMode};
/// # use tempfile::tempdir;
/// # let temp = tempdir().unwrap();
/// # let path = temp.path().join("image.pif");
///
/// // Create a new PIF asset
/// let mut manager = PifAssetManager::create_new(&path, 1024, 768, SaveMode::Directory).unwrap();
///
/// // Access the manifest
/// println!("Canvas: {}x{}", manager.manifest().canvas.width, manager.manifest().canvas.height);
/// ```
pub struct PifAssetManager {
    root_path: PathBuf,
    manifest: PifManifest,
    save_mode: SaveMode,
}

impl PifAssetManager {
    /// Creates a brand new, empty .pif structure and initializes the storage layout.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the PIF asset will be created
    /// * `width` - Canvas width in pixels
    /// * `height` - Canvas height in pixels
    /// * `save_mode` - Whether to use directory bundle or single-file archive mode
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pulsar_image_format::{PifAssetManager, SaveMode};
    /// # use tempfile::tempdir;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let temp = tempdir()?;
    /// # let path = temp.path().join("image.pif");
    ///
    /// let manager = PifAssetManager::create_new(&path, 1024, 768, SaveMode::Directory)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_new<P: AsRef<Path>>(
        path: P,
        width: u32,
        height: u32,
        save_mode: SaveMode,
    ) -> Result<Self> {
        let root_path = path.as_ref().to_path_buf();
        let manifest = PifManifest::new(width, height);

        match save_mode {
            SaveMode::Directory => directory::create_bundle(&root_path, &manifest)?,
            SaveMode::Archive => archive::create_archive(&root_path, &manifest)?,
        }

        Ok(Self {
            root_path,
            manifest,
            save_mode,
        })
    }

    /// Opens an existing .pif directory bundle or archive file and parses the structural manifest.
    ///
    /// The save mode is automatically detected based on whether the path is a directory or file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pulsar_image_format::PifAssetManager;
    /// # use pulsar_image_format::SaveMode;
    /// # use tempfile::tempdir;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let temp = tempdir()?;
    /// # let path = temp.path().join("image.pif");
    /// # PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory)?;
    ///
    /// let manager = PifAssetManager::open(&path)?;
    /// println!("Canvas size: {}x{}", manager.manifest().canvas.width, manager.manifest().canvas.height);
    /// # Ok(())
    /// # }
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let root_path = path.as_ref().to_path_buf();

        let (manifest, save_mode) = if root_path.is_dir() {
            let manifest = directory::load_manifest(&root_path)?;
            (manifest, SaveMode::Directory)
        } else {
            let manifest = archive::load_manifest(&root_path)?;
            (manifest, SaveMode::Archive)
        };

        Ok(Self {
            root_path,
            manifest,
            save_mode,
        })
    }

    /// Returns a read-only reference to the underlying structural manifest metadata
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pulsar_image_format::{PifAssetManager, SaveMode};
    /// # use tempfile::tempdir;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let temp = tempdir()?;
    /// # let path = temp.path().join("image.pif");
    /// # let manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory)?;
    /// let manifest = manager.manifest();
    /// println!("PIF version: {}", manifest.pif_version);
    /// # Ok(())
    /// # }
    /// ```
    pub fn manifest(&self) -> &PifManifest {
        &self.manifest
    }

    /// Returns a mutable reference for modifying vector nodes, changing layer orders, or layer visibility
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pulsar_image_format::{PifAssetManager, SaveMode, model::Layer};
    /// # use tempfile::tempdir;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let temp = tempdir()?;
    /// # let path = temp.path().join("image.pif");
    /// let mut manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory)?;
    ///
    /// // Add a new raster layer
    /// manager.manifest_mut().layers.push(Layer::Raster {
    ///     id: "background".to_string(),
    ///     name: "Background".to_string(),
    ///     visible: true,
    ///     opacity: 1.0,
    ///     blend_mode: "normal".to_string(),
    ///     tile_size: 256,
    ///     tiles: Default::default(),
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn manifest_mut(&mut self) -> &mut PifManifest {
        &mut self.manifest
    }

    /// Returns the current save mode (Directory or Archive)
    pub fn save_mode(&self) -> SaveMode {
        self.save_mode
    }

    /// Fetches the raw RGBA8888 pixel vector for a specific tile.
    ///
    /// **Optimization**: If the tile is unallocated (sparse), returns a completely
    /// transparent buffer without throwing an error, saving CPU cycles.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - ID of the raster layer
    /// * `tile_x` - Tile X coordinate
    /// * `tile_y` - Tile Y coordinate
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pulsar_image_format::{PifAssetManager, SaveMode, model::Layer};
    /// # use tempfile::tempdir;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let temp = tempdir()?;
    /// # let path = temp.path().join("image.pif");
    /// # let mut manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory)?;
    /// # manager.manifest_mut().layers.push(Layer::Raster {
    /// #     id: "layer1".to_string(), name: "Layer 1".to_string(),
    /// #     visible: true, opacity: 1.0, blend_mode: "normal".to_string(),
    /// #     tile_size: 256, tiles: Default::default(),
    /// # });
    ///
    /// let pixels = manager.load_raster_tile("layer1", 0, 0)?;
    /// assert_eq!(pixels.len(), 256 * 256 * 4);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_raster_tile(
        &self,
        layer_id: &str,
        tile_x: u32,
        tile_y: u32,
    ) -> Result<Vec<u8>> {
        // Find the layer
        let layer = self
            .manifest
            .find_layer(layer_id)
            .ok_or_else(|| PifError::LayerNotFound(layer_id.to_string()))?;

        // Verify it's a raster layer
        let (tile_size, tiles) = match layer {
            Layer::Raster { tile_size, tiles, .. } => (tile_size, tiles),
            _ => return Err(PifError::InvalidLayerType),
        };

        // Check bounds
        let (max_tile_x, max_tile_y) = tile::max_tile_coords(
            self.manifest.canvas.width,
            self.manifest.canvas.height,
            *tile_size,
        );

        if tile_x >= max_tile_x || tile_y >= max_tile_y {
            return Err(PifError::OutOfBounds {
                x: tile_x,
                y: tile_y,
            });
        }

        // Check if tile exists
        let tile_key = tile::tile_key(tile_x, tile_y);

        if let Some(tile_path_str) = tiles.get(&tile_key) {
            // Load tile from storage
            match self.save_mode {
                SaveMode::Directory => {
                    directory::load_tile(&self.root_path, tile_path_str, *tile_size)
                }
                SaveMode::Archive => {
                    archive::load_tile(&self.root_path, tile_path_str, *tile_size)
                }
            }
        } else {
            // Return transparent tile (sparse optimization)
            Ok(tile::empty_tile(*tile_size))
        }
    }

    /// Atomically commits modified pixel grids to disk via QOI encoding and updates the manifest.
    ///
    /// This operation is transactional - if any write fails, the manifest will not be corrupted.
    ///
    /// # Arguments
    ///
    /// * `dirty_tiles` - Map of (layer_id, tile_x, tile_y) to RGBA pixel buffers
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pulsar_image_format::{PifAssetManager, SaveMode, model::Layer};
    /// # use std::collections::HashMap;
    /// # use tempfile::tempdir;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let temp = tempdir()?;
    /// # let path = temp.path().join("image.pif");
    /// # let mut manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory)?;
    /// # manager.manifest_mut().layers.push(Layer::Raster {
    /// #     id: "layer1".to_string(), name: "Layer 1".to_string(),
    /// #     visible: true, opacity: 1.0, blend_mode: "normal".to_string(),
    /// #     tile_size: 256, tiles: Default::default(),
    /// # });
    ///
    /// let mut modified_pixels = vec![0u8; 256 * 256 * 4];
    /// // Paint a red pixel in the top-left corner
    /// modified_pixels[0..4].copy_from_slice(&[255, 0, 0, 255]);
    ///
    /// let mut session_data = HashMap::new();
    /// session_data.insert(("layer1".to_string(), 0, 0), modified_pixels);
    /// manager.commit_changes(session_data)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn commit_changes(
        &mut self,
        dirty_tiles: HashMap<(String, u32, u32), Vec<u8>>,
    ) -> Result<()> {
        // Process each dirty tile and update manifest
        for ((layer_id, tile_x, tile_y), pixels) in &dirty_tiles {
            // Find the layer
            let layer = self
                .manifest
                .find_layer_mut(layer_id)
                .ok_or_else(|| PifError::LayerNotFound(layer_id.clone()))?;

            // Verify it's a raster layer and get tile info
            let (tile_size, tiles) = match layer {
                Layer::Raster { tile_size, tiles, .. } => (tile_size, tiles),
                _ => return Err(PifError::InvalidLayerType),
            };

            let tile_key = tile::tile_key(*tile_x, *tile_y);
            let tile_path_str = tile::tile_path(layer_id, *tile_x, *tile_y);

            // Update manifest tile mapping
            tiles.insert(tile_key, tile_path_str.clone());

            // Save tile data
            match self.save_mode {
                SaveMode::Directory => {
                    directory::save_tile(&self.root_path, &tile_path_str, pixels, *tile_size)?;
                }
                SaveMode::Archive => {
                    // Archive mode deferred - will rebuild entire archive after loop
                }
            }
        }

        // Save updated manifest
        match self.save_mode {
            SaveMode::Directory => {
                directory::save_manifest(&self.root_path, &self.manifest)?;
            }
            SaveMode::Archive => {
                // Rebuild the entire archive with all tiles
                let mut archive_tiles = HashMap::new();
                for ((layer_id, tile_x, tile_y), pixels) in dirty_tiles {
                    let tile_path_str = tile::tile_path(&layer_id, tile_x, tile_y);
                    archive_tiles.insert(tile_path_str, pixels);
                }

                // Get tile size from first layer (assumes all tiles same size for now)
                let tile_size = self
                    .manifest
                    .layers
                    .iter()
                    .find_map(|l| match l {
                        Layer::Raster { tile_size, .. } => Some(*tile_size),
                        _ => None,
                    })
                    .unwrap_or(256);

                archive::rebuild_archive(&self.root_path, &self.manifest, &archive_tiles, tile_size)?;
            }
        }

        Ok(())
    }

    /// Composites and exports the entire image to a single file format.
    ///
    /// Currently supported formats:
    /// - `"svg"` - Exports vector layers as SVG (raster layers are ignored)
    /// - `"png"` - Exports as PNG (not yet implemented)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use pulsar_image_format::{PifAssetManager, SaveMode};
    /// # use tempfile::tempdir;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let temp = tempdir()?;
    /// # let path = temp.path().join("image.pif");
    /// # let manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory)?;
    ///
    /// manager.bake("output.svg", "svg")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn bake<P: AsRef<Path>>(&self, output_file: P, target_format: &str) -> Result<()> {
        match target_format {
            "svg" => bake::export_svg(&self.manifest, output_file),
            "png" => bake::export_png(&self.manifest, output_file),
            _ => Err(PifError::QoiCodec(format!(
                "Unsupported bake format: {}",
                target_format
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_directory_bundle() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");

        let manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory).unwrap();

        assert!(path.exists());
        assert!(path.is_dir());
        assert_eq!(manager.manifest().canvas.width, 512);
        assert_eq!(manager.manifest().canvas.height, 512);
    }

    #[test]
    fn test_create_archive() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");

        let manager = PifAssetManager::create_new(&path, 256, 256, SaveMode::Archive).unwrap();

        assert!(path.exists());
        assert!(path.is_file());
        assert_eq!(manager.save_mode(), SaveMode::Archive);
    }

    #[test]
    fn test_open_directory() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");

        PifAssetManager::create_new(&path, 1024, 768, SaveMode::Directory).unwrap();
        let manager = PifAssetManager::open(&path).unwrap();

        assert_eq!(manager.save_mode(), SaveMode::Directory);
        assert_eq!(manager.manifest().canvas.width, 1024);
    }

    #[test]
    fn test_load_sparse_tile() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");

        let mut manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory).unwrap();

        manager.manifest_mut().layers.push(Layer::Raster {
            id: "test_layer".to_string(),
            name: "Test Layer".to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            tile_size: 256,
            tiles: HashMap::new(),
        });

        let pixels = manager.load_raster_tile("test_layer", 0, 0).unwrap();
        assert_eq!(pixels.len(), 256 * 256 * 4);
        assert!(pixels.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_commit_tile_directory() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");

        let mut manager = PifAssetManager::create_new(&path, 512, 512, SaveMode::Directory).unwrap();

        manager.manifest_mut().layers.push(Layer::Raster {
            id: "layer1".to_string(),
            name: "Layer 1".to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            tile_size: 256,
            tiles: HashMap::new(),
        });

        let mut pixels = vec![0u8; 256 * 256 * 4];
        pixels[0..4].copy_from_slice(&[255, 0, 0, 255]);

        let mut dirty = HashMap::new();
        dirty.insert(("layer1".to_string(), 0, 0), pixels);

        manager.commit_changes(dirty).unwrap();

        let loaded = manager.load_raster_tile("layer1", 0, 0).unwrap();
        assert_eq!(loaded[0..4], [255, 0, 0, 255]);
    }
}
