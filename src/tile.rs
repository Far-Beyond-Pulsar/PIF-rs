//! Tile operations for loading, saving, and encoding raster data
//!
//! This module handles all tile-related operations including QOI encoding/decoding,
//! sparse tile optimization, and tile coordinate management.

use crate::error::{PifError, Result};

/// Size of a standard tile in pixels (256x256)
pub const TILE_SIZE: u32 = 256;

/// Number of bytes per pixel (RGBA8888)
pub const BYTES_PER_PIXEL: usize = 4;

/// Total bytes in a standard tile
pub const TILE_BUFFER_SIZE: usize = (TILE_SIZE as usize) * (TILE_SIZE as usize) * BYTES_PER_PIXEL;

/// Encodes raw RGBA pixel data into QOI format
///
/// # Arguments
///
/// * `pixels` - Raw RGBA8888 pixel buffer
/// * `tile_size` - Size of the tile in pixels (width and height, must be square)
///
/// # Returns
///
/// Compressed QOI-encoded byte vector
///
/// # Examples
///
/// ```rust
/// use pulsar_image_format::tile;
///
/// let pixels = vec![0u8; 256 * 256 * 4];
/// let encoded = tile::encode_qoi(&pixels, 256).unwrap();
/// ```
pub fn encode_qoi(pixels: &[u8], tile_size: u32) -> Result<Vec<u8>> {
    let expected_size = (tile_size as usize) * (tile_size as usize) * BYTES_PER_PIXEL;
    
    if pixels.len() != expected_size {
        return Err(PifError::InvalidTileSize {
            expected: expected_size,
            actual: pixels.len(),
        });
    }

    qoi::encode_to_vec(pixels, tile_size, tile_size)
        .map_err(|e| PifError::QoiCodec(e.to_string()))
}

/// Decodes QOI-formatted data back into raw RGBA pixels
///
/// # Arguments
///
/// * `qoi_data` - QOI-encoded byte buffer
///
/// # Returns
///
/// Tuple of (decoded pixels, width, height)
///
/// # Examples
///
/// ```rust
/// use pulsar_image_format::tile;
///
/// # let pixels = vec![0u8; 256 * 256 * 4];
/// # let qoi_data = tile::encode_qoi(&pixels, 256).unwrap();
/// let (decoded, width, height) = tile::decode_qoi(&qoi_data).unwrap();
/// assert_eq!(width, 256);
/// assert_eq!(height, 256);
/// ```
pub fn decode_qoi(qoi_data: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let (header, pixels) = qoi::decode_to_vec(qoi_data)
        .map_err(|e| PifError::QoiCodec(e.to_string()))?;
    
    Ok((pixels, header.width, header.height))
}

/// Creates a tile key string from coordinates
///
/// # Examples
///
/// ```rust
/// use pulsar_image_format::tile::tile_key;
///
/// assert_eq!(tile_key(0, 0), "0_0");
/// assert_eq!(tile_key(3, 7), "3_7");
/// ```
pub fn tile_key(x: u32, y: u32) -> String {
    format!("{}_{}", x, y)
}

/// Creates a tile file path for a given layer and tile coordinates
///
/// # Examples
///
/// ```rust
/// use pulsar_image_format::tile::tile_path;
///
/// assert_eq!(tile_path("layer1", 0, 0), ".raster/layer_layer1_0_0.qoi");
/// ```
pub fn tile_path(layer_id: &str, x: u32, y: u32) -> String {
    format!(".raster/layer_{}_{}.qoi", layer_id, tile_key(x, y))
}

/// Creates an empty (transparent) tile buffer
///
/// Returns a vector of zeros representing a fully transparent RGBA tile.
///
/// # Arguments
///
/// * `tile_size` - Size of the tile in pixels
///
/// # Examples
///
/// ```rust
/// use pulsar_image_format::tile;
///
/// let empty = tile::empty_tile(256);
/// assert_eq!(empty.len(), 256 * 256 * 4);
/// assert!(empty.iter().all(|&b| b == 0));
/// ```
pub fn empty_tile(tile_size: u32) -> Vec<u8> {
    vec![0; (tile_size as usize) * (tile_size as usize) * BYTES_PER_PIXEL]
}

/// Checks if a tile is completely transparent (sparse)
///
/// This is used for optimization - sparse tiles don't need to be saved.
///
/// # Examples
///
/// ```rust
/// use pulsar_image_format::tile;
///
/// let empty = vec![0u8; 256 * 256 * 4];
/// assert!(tile::is_sparse(&empty));
///
/// let mut painted = empty.clone();
/// painted[0] = 255;
/// assert!(!tile::is_sparse(&painted));
/// ```
pub fn is_sparse(pixels: &[u8]) -> bool {
    pixels.iter().all(|&b| b == 0)
}

/// Calculates the maximum tile coordinates for a canvas size
///
/// # Arguments
///
/// * `width` - Canvas width in pixels
/// * `height` - Canvas height in pixels  
/// * `tile_size` - Tile size in pixels
///
/// # Returns
///
/// Tuple of (max_tile_x, max_tile_y)
///
/// # Examples
///
/// ```rust
/// use pulsar_image_format::tile::max_tile_coords;
///
/// let (max_x, max_y) = max_tile_coords(1024, 768, 256);
/// assert_eq!(max_x, 4); // ceil(1024 / 256)
/// assert_eq!(max_y, 3); // ceil(768 / 256)
/// ```
pub fn max_tile_coords(width: u32, height: u32, tile_size: u32) -> (u32, u32) {
    let max_x = (width + tile_size - 1) / tile_size;
    let max_y = (height + tile_size - 1) / tile_size;
    (max_x, max_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = vec![255u8; 256 * 256 * 4];
        let encoded = encode_qoi(&original, 256).unwrap();
        let (decoded, w, h) = decode_qoi(&encoded).unwrap();
        
        assert_eq!(w, 256);
        assert_eq!(h, 256);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_tile_key() {
        assert_eq!(tile_key(0, 0), "0_0");
        assert_eq!(tile_key(10, 20), "10_20");
    }

    #[test]
    fn test_tile_path() {
        assert_eq!(tile_path("bg", 0, 0), ".raster/layer_bg_0_0.qoi");
        assert_eq!(tile_path("fg", 3, 5), ".raster/layer_fg_3_5.qoi");
    }

    #[test]
    fn test_is_sparse() {
        let sparse = vec![0u8; 1024];
        assert!(is_sparse(&sparse));

        let mut not_sparse = sparse.clone();
        not_sparse[100] = 1;
        assert!(!is_sparse(&not_sparse));
    }

    #[test]
    fn test_max_tile_coords() {
        assert_eq!(max_tile_coords(256, 256, 256), (1, 1));
        assert_eq!(max_tile_coords(257, 256, 256), (2, 1));
        assert_eq!(max_tile_coords(512, 512, 256), (2, 2));
        assert_eq!(max_tile_coords(1000, 800, 256), (4, 4));
    }
}
