use thiserror::Error;

/// Errors that can occur when working with PIF assets
#[derive(Error, Debug)]
pub enum PifError {
    /// I/O operation failed
    #[error("I/O error encountered: {0}")]
    Io(#[from] std::io::Error),
    
    /// JSON serialization/deserialization failed
    #[error("JSON Serialization/Deserialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// QOI codec error
    #[error("QOI codec error: {0}")]
    QoiCodec(String),
    
    /// Layer not found in manifest
    #[error("Layer with ID '{0}' not found in manifest")]
    LayerNotFound(String),
    
    /// Invalid layer type for the requested operation
    #[error("Invalid layer type for requested operation")]
    InvalidLayerType,
    
    /// Coordinate exceeds canvas boundaries
    #[error("Coordinate ({x}, {y}) exceeds canvas boundaries")]
    OutOfBounds { x: u32, y: u32 },
    
    /// Tile buffer has incorrect size
    #[error("Tile buffer size mismatch: expected {expected}, got {actual}")]
    InvalidTileSize { expected: usize, actual: usize },
    
    /// ZIP archive error
    #[error("ZIP archive error: {0}")]
    ZipError(#[from] zip::result::ZipError),
}

/// Convenience type alias for Results using PifError
pub type Result<T> = std::result::Result<T, PifError>;
