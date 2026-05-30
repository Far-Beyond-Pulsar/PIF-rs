use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Color space specification for the canvas
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColorSpace {
    /// Standard RGB color space with gamma correction
    #[serde(rename = "sRGB")]
    SRgb,
    /// Linear color space without gamma correction
    Linear,
}

/// Canvas configuration defining the image dimensions and color space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasConfig {
    /// Width of the canvas in pixels
    pub width: u32,
    /// Height of the canvas in pixels
    pub height: u32,
    /// Color space used for rendering
    pub color_space: ColorSpace,
}

/// Vector shape elements that can be rendered
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum VectorElement {
    /// Rectangle shape
    Rect {
        /// X coordinate of the top-left corner
        x: f32,
        /// Y coordinate of the top-left corner
        y: f32,
        /// Width of the rectangle
        w: f32,
        /// Height of the rectangle
        h: f32,
        /// Fill color in CSS color format
        fill: String,
    },
    /// Circle shape
    Circle {
        /// X coordinate of the center
        cx: f32,
        /// Y coordinate of the center
        cy: f32,
        /// Radius of the circle
        r: f32,
        /// Fill color in CSS color format
        fill: String,
    },
    /// SVG path
    Path {
        /// SVG path data string
        data: String,
        /// Stroke color in CSS color format
        stroke: String,
        /// Stroke width in pixels
        stroke_width: f32,
    },
}

/// A layer in the PIF document, either vector or raster-based
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Layer {
    /// Vector layer containing scalable shapes
    Vector {
        /// Unique identifier for the layer
        id: String,
        /// Human-readable layer name
        name: String,
        /// Whether the layer is visible
        visible: bool,
        /// Layer opacity (0.0 = transparent, 1.0 = opaque)
        opacity: f32,
        /// Blend mode for compositing
        blend_mode: String,
        /// Vector elements in this layer
        elements: Vec<VectorElement>,
    },
    /// Raster layer containing pixel data in tiles
    Raster {
        /// Unique identifier for the layer
        id: String,
        /// Human-readable layer name
        name: String,
        /// Whether the layer is visible
        visible: bool,
        /// Layer opacity (0.0 = transparent, 1.0 = opaque)
        opacity: f32,
        /// Blend mode for compositing
        blend_mode: String,
        /// Size of each tile in pixels (typically 256)
        tile_size: u32,
        /// Maps tile coordinates "x_y" (e.g., "0_2") to relative file paths
        tiles: HashMap<String, String>,
    },
}

impl Layer {
    /// Returns the layer's unique identifier
    pub fn id(&self) -> &str {
        match self {
            Layer::Vector { id, .. } => id,
            Layer::Raster { id, .. } => id,
        }
    }

    /// Returns the layer's name
    pub fn name(&self) -> &str {
        match self {
            Layer::Vector { name, .. } => name,
            Layer::Raster { name, .. } => name,
        }
    }

    /// Returns whether the layer is visible
    pub fn is_visible(&self) -> bool {
        match self {
            Layer::Vector { visible, .. } => *visible,
            Layer::Raster { visible, .. } => *visible,
        }
    }

    /// Returns the layer's opacity
    pub fn opacity(&self) -> f32 {
        match self {
            Layer::Vector { opacity, .. } => *opacity,
            Layer::Raster { opacity, .. } => *opacity,
        }
    }
}

/// Root manifest structure for a PIF document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PifManifest {
    /// Version of the PIF format specification
    pub pif_version: String,
    /// Canvas configuration
    pub canvas: CanvasConfig,
    /// Ordered list of layers (bottom to top)
    pub layers: Vec<Layer>,
}

impl PifManifest {
    /// Creates a new manifest with default settings
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            pif_version: "1.0".to_string(),
            canvas: CanvasConfig {
                width,
                height,
                color_space: ColorSpace::SRgb,
            },
            layers: Vec::new(),
        }
    }

    /// Finds a layer by its ID
    pub fn find_layer(&self, id: &str) -> Option<&Layer> {
        self.layers.iter().find(|layer| layer.id() == id)
    }

    /// Finds a mutable layer by its ID
    pub fn find_layer_mut(&mut self, id: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|layer| layer.id() == id)
    }
}
