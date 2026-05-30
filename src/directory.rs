//! Directory bundle operations for Git-friendly storage
//!
//! This module handles reading and writing PIF assets in directory bundle format,
//! where the manifest is stored as JSON and tiles are stored as individual QOI files.

use crate::error::{PifError, Result};
use crate::model::PifManifest;
use crate::tile;
use std::fs;
use std::path::Path;

/// Creates a new directory bundle structure
///
/// # Arguments
///
/// * `path` - Root directory path for the bundle
/// * `manifest` - Initial manifest to save
pub fn create_bundle<P: AsRef<Path>>(path: P, manifest: &PifManifest) -> Result<()> {
    let root = path.as_ref();
    
    // Create directory structure
    fs::create_dir_all(root)?;
    let raster_dir = root.join(".raster");
    fs::create_dir_all(&raster_dir)?;
    
    // Write manifest atomically
    save_manifest(root, manifest)?;
    
    Ok(())
}

/// Loads a manifest from a directory bundle
///
/// # Arguments
///
/// * `path` - Root directory path of the bundle
pub fn load_manifest<P: AsRef<Path>>(path: P) -> Result<PifManifest> {
    let manifest_path = path.as_ref().join("manifest.json");
    let manifest_data = fs::read_to_string(&manifest_path)?;
    let manifest: PifManifest = serde_json::from_str(&manifest_data)?;
    Ok(manifest)
}

/// Saves a manifest to a directory bundle atomically
///
/// Uses a temporary file and atomic rename to prevent corruption.
///
/// # Arguments
///
/// * `path` - Root directory path of the bundle
/// * `manifest` - Manifest to save
pub fn save_manifest<P: AsRef<Path>>(path: P, manifest: &PifManifest) -> Result<()> {
    let manifest_path = path.as_ref().join("manifest.json");
    let temp_path = manifest_path.with_extension("tmp");
    
    let manifest_json = serde_json::to_string_pretty(manifest)?;
    fs::write(&temp_path, manifest_json)?;
    fs::rename(&temp_path, &manifest_path)?;
    
    Ok(())
}

/// Loads a tile from a directory bundle
///
/// # Arguments
///
/// * `root_path` - Root directory of the bundle
/// * `tile_path` - Relative path to the tile file
/// * `tile_size` - Expected tile size in pixels
pub fn load_tile<P: AsRef<Path>>(
    root_path: P,
    tile_path: &str,
    tile_size: u32,
) -> Result<Vec<u8>> {
    let full_path = root_path.as_ref().join(tile_path);
    let qoi_data = fs::read(full_path)?;
    let (pixels, width, height) = tile::decode_qoi(&qoi_data)?;
    
    // Verify dimensions
    if width != tile_size || height != tile_size {
        return Err(PifError::InvalidTileSize {
            expected: (tile_size * tile_size * 4) as usize,
            actual: (width * height * 4) as usize,
        });
    }
    
    Ok(pixels)
}

/// Saves a tile to a directory bundle
///
/// # Arguments
///
/// * `root_path` - Root directory of the bundle
/// * `tile_path` - Relative path where the tile should be saved
/// * `pixels` - Raw RGBA pixel data
/// * `tile_size` - Tile size in pixels
pub fn save_tile<P: AsRef<Path>>(
    root_path: P,
    tile_path: &str,
    pixels: &[u8],
    tile_size: u32,
) -> Result<()> {
    let full_path = root_path.as_ref().join(tile_path);
    
    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let qoi_data = tile::encode_qoi(pixels, tile_size)?;
    fs::write(full_path, qoi_data)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_bundle() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");
        
        let manifest = PifManifest::new(512, 512);
        create_bundle(&path, &manifest).unwrap();
        
        assert!(path.exists());
        assert!(path.is_dir());
        assert!(path.join("manifest.json").exists());
        assert!(path.join(".raster").exists());
    }

    #[test]
    fn test_save_load_manifest() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");
        fs::create_dir_all(&path).unwrap();
        
        let manifest = PifManifest::new(1024, 768);
        save_manifest(&path, &manifest).unwrap();
        
        let loaded = load_manifest(&path).unwrap();
        assert_eq!(loaded.canvas.width, 1024);
        assert_eq!(loaded.canvas.height, 768);
    }

    #[test]
    fn test_save_load_tile() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");
        fs::create_dir_all(&path).unwrap();
        fs::create_dir_all(&path.join(".raster")).unwrap();
        
        let mut pixels = vec![0u8; 256 * 256 * 4];
        pixels[0..4].copy_from_slice(&[255, 128, 64, 255]);
        
        save_tile(&path, ".raster/test.qoi", &pixels, 256).unwrap();
        let loaded = load_tile(&path, ".raster/test.qoi", 256).unwrap();
        
        assert_eq!(loaded[0..4], [255, 128, 64, 255]);
    }
}
