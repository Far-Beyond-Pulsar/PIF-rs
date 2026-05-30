//! ZIP archive operations for single-file distribution
//!
//! This module handles reading and writing PIF assets in single-file ZIP archive format,
//! which is more convenient for distribution but less Git-friendly than directory bundles.

use crate::error::{PifError, Result};
use crate::model::PifManifest;
use crate::tile;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Creates a new ZIP archive with an initial manifest
///
/// # Arguments
///
/// * `path` - Path where the archive will be created
/// * `manifest` - Initial manifest to include
pub fn create_archive<P: AsRef<Path>>(path: P, manifest: &PifManifest) -> Result<()> {
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated);
    
    // Write manifest
    zip.start_file("manifest.json", options)?;
    let manifest_json = serde_json::to_string_pretty(manifest)?;
    zip.write_all(manifest_json.as_bytes())?;
    
    zip.finish()?;
    Ok(())
}

/// Loads a manifest from a ZIP archive
///
/// # Arguments
///
/// * `path` - Path to the archive file
pub fn load_manifest<P: AsRef<Path>>(path: P) -> Result<PifManifest> {
    let file = File::open(path)?;
    let mut zip = ZipArchive::new(file)?;
    
    let mut manifest_file = zip.by_name("manifest.json")?;
    let mut manifest_data = String::new();
    manifest_file.read_to_string(&mut manifest_data)?;
    
    let manifest: PifManifest = serde_json::from_str(&manifest_data)?;
    Ok(manifest)
}

/// Loads a tile from a ZIP archive
///
/// # Arguments
///
/// * `archive_path` - Path to the archive file
/// * `tile_path` - Path to the tile within the archive
/// * `tile_size` - Expected tile size in pixels
pub fn load_tile<P: AsRef<Path>>(
    archive_path: P,
    tile_path: &str,
    tile_size: u32,
) -> Result<Vec<u8>> {
    let file = File::open(archive_path)?;
    let mut zip = ZipArchive::new(file)?;
    
    let mut tile_file = zip.by_name(tile_path)?;
    let mut qoi_data = Vec::new();
    tile_file.read_to_end(&mut qoi_data)?;
    
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

/// Rebuilds an entire archive with updated data
///
/// This is necessary because ZIP files don't support in-place modification.
/// The function creates a new archive with all existing data plus any changes.
///
/// # Arguments
///
/// * `archive_path` - Path to the existing archive
/// * `manifest` - Updated manifest to write
/// * `new_tiles` - Map of tile paths to pixel data for tiles to add/update
pub fn rebuild_archive<P: AsRef<Path>>(
    archive_path: P,
    manifest: &PifManifest,
    new_tiles: &std::collections::HashMap<String, Vec<u8>>,
    tile_size: u32,
) -> Result<()> {
    let path = archive_path.as_ref();
    let temp_path = path.with_extension("tmp.zip");
    
    {
        let temp_file = File::create(&temp_path)?;
        let mut new_zip = ZipWriter::new(temp_file);
        
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(CompressionMethod::Deflated);
        
        // Write updated manifest
        new_zip.start_file("manifest.json", options)?;
        let manifest_json = serde_json::to_string_pretty(manifest)?;
        new_zip.write_all(manifest_json.as_bytes())?;
        
        // Copy existing files (except manifest and tiles being replaced)
        let old_file = File::open(path)?;
        let mut old_zip = ZipArchive::new(old_file)?;
        
        for i in 0..old_zip.len() {
            let mut file = old_zip.by_index(i)?;
            let name = file.name().to_string();
            
            // Skip manifest (already wrote updated version)
            if name == "manifest.json" {
                continue;
            }
            
            // Skip tiles that will be replaced
            if new_tiles.contains_key(&name) {
                continue;
            }
            
            // Copy existing file
            new_zip.start_file(&name, options)?;
            std::io::copy(&mut file, &mut new_zip)?;
        }
        
        // Write new/updated tiles
        for (tile_path, pixels) in new_tiles {
            let qoi_data = tile::encode_qoi(pixels, tile_size)?;
            new_zip.start_file(tile_path, options)?;
            new_zip.write_all(&qoi_data)?;
        }
        
        new_zip.finish()?;
    }
    
    // Atomic replacement
    std::fs::rename(&temp_path, path)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_create_archive() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");
        
        let manifest = PifManifest::new(512, 512);
        create_archive(&path, &manifest).unwrap();
        
        assert!(path.exists());
        assert!(path.is_file());
    }

    #[test]
    fn test_load_manifest() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");
        
        let manifest = PifManifest::new(1024, 768);
        create_archive(&path, &manifest).unwrap();
        
        let loaded = load_manifest(&path).unwrap();
        assert_eq!(loaded.canvas.width, 1024);
        assert_eq!(loaded.canvas.height, 768);
    }

    #[test]
    fn test_rebuild_with_tiles() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.pif");
        
        let manifest = PifManifest::new(256, 256);
        create_archive(&path, &manifest).unwrap();
        
        // Add a tile
        let mut pixels = vec![0u8; 256 * 256 * 4];
        pixels[0..4].copy_from_slice(&[255, 0, 0, 255]);
        
        let mut new_tiles = HashMap::new();
        new_tiles.insert(".raster/test.qoi".to_string(), pixels.clone());
        
        rebuild_archive(&path, &manifest, &new_tiles, 256).unwrap();
        
        // Verify tile was saved
        let loaded = load_tile(&path, ".raster/test.qoi", 256).unwrap();
        assert_eq!(loaded[0..4], [255, 0, 0, 255]);
    }
}
