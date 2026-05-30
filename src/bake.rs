//! Export and compositing operations for baking PIF assets to other formats
//!
//! This module provides functionality to export PIF assets to standard formats
//! like PNG and SVG for use outside the Pulsar engine ecosystem.

use crate::error::{PifError, Result};
use crate::model::{Layer, PifManifest, VectorElement};
use std::fs;
use std::path::Path;

/// Exports vector layers to SVG format
///
/// Raster layers are ignored in SVG export. Only visible vector layers are included.
///
/// # Arguments
///
/// * `manifest` - PIF manifest containing the layers
/// * `output_path` - Path where the SVG file will be written
///
/// # Examples
///
/// ```rust,no_run
/// use pulsar_image_format::bake;
/// # use pulsar_image_format::model::PifManifest;
/// # let manifest = PifManifest::new(512, 512);
///
/// bake::export_svg(&manifest, "output.svg").unwrap();
/// ```
pub fn export_svg<P: AsRef<Path>>(manifest: &PifManifest, output_path: P) -> Result<()> {
    let mut svg = String::new();
    
    // SVG header
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}">"#,
        manifest.canvas.width, manifest.canvas.height
    ));
    svg.push('\n');
    
    // Process each vector layer
    for layer in &manifest.layers {
        if let Layer::Vector { visible, elements, opacity, .. } = layer {
            if !visible {
                continue;
            }
            
            // Apply layer opacity via group if needed
            if *opacity < 1.0 {
                svg.push_str(&format!(r#"  <g opacity="{}">"#, opacity));
                svg.push('\n');
            }
            
            for element in elements {
                render_vector_element(&mut svg, element);
            }
            
            if *opacity < 1.0 {
                svg.push_str("  </g>\n");
            }
        }
    }
    
    svg.push_str("</svg>\n");
    
    fs::write(output_path, svg)?;
    Ok(())
}

/// Renders a single vector element to SVG markup
fn render_vector_element(svg: &mut String, element: &VectorElement) {
    match element {
        VectorElement::Rect { x, y, w, h, fill } => {
            svg.push_str(&format!(
                r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
                x, y, w, h, fill
            ));
            svg.push('\n');
        }
        VectorElement::Circle { cx, cy, r, fill } => {
            svg.push_str(&format!(
                r#"  <circle cx="{}" cy="{}" r="{}" fill="{}"/>"#,
                cx, cy, r, fill
            ));
            svg.push('\n');
        }
        VectorElement::Path { data, stroke, stroke_width } => {
            svg.push_str(&format!(
                r#"  <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
                data, stroke, stroke_width
            ));
            svg.push('\n');
        }
    }
}

/// Exports the entire canvas to PNG format (placeholder)
///
/// **Note**: PNG export is not yet implemented. This requires implementing
/// a full compositing pipeline with layer blending.
///
/// # Future Implementation
///
/// The PNG export will:
/// 1. Composite all raster layers bottom-to-top
/// 2. Apply blend modes and opacity
/// 3. Encode the result as PNG
///
/// This requires additional dependencies like `png` or `image` crates.
pub fn export_png<P: AsRef<Path>>(_manifest: &PifManifest, _output_path: P) -> Result<()> {
    Err(PifError::QoiCodec(
        "PNG export not yet implemented - requires compositing engine".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Layer, VectorElement};
    use tempfile::tempdir;

    #[test]
    fn test_export_svg_basic() {
        let temp = tempdir().unwrap();
        let output = temp.path().join("output.svg");
        
        let mut manifest = PifManifest::new(200, 200);
        
        manifest.layers.push(Layer::Vector {
            id: "shapes".to_string(),
            name: "Shapes".to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            elements: vec![
                VectorElement::Rect {
                    x: 10.0,
                    y: 10.0,
                    w: 50.0,
                    h: 50.0,
                    fill: "#ff0000".to_string(),
                },
                VectorElement::Circle {
                    cx: 100.0,
                    cy: 100.0,
                    r: 30.0,
                    fill: "#00ff00".to_string(),
                },
            ],
        });
        
        export_svg(&manifest, &output).unwrap();
        
        let svg_content = fs::read_to_string(&output).unwrap();
        assert!(svg_content.contains("rect"));
        assert!(svg_content.contains("circle"));
        assert!(svg_content.contains("#ff0000"));
        assert!(svg_content.contains("#00ff00"));
        assert!(svg_content.contains(r#"width="200""#));
        assert!(svg_content.contains(r#"height="200""#));
    }

    #[test]
    fn test_export_svg_with_opacity() {
        let temp = tempdir().unwrap();
        let output = temp.path().join("output.svg");
        
        let mut manifest = PifManifest::new(100, 100);
        
        manifest.layers.push(Layer::Vector {
            id: "semi".to_string(),
            name: "Semi-transparent".to_string(),
            visible: true,
            opacity: 0.5,
            blend_mode: "normal".to_string(),
            elements: vec![
                VectorElement::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 50.0,
                    h: 50.0,
                    fill: "#0000ff".to_string(),
                },
            ],
        });
        
        export_svg(&manifest, &output).unwrap();
        
        let svg_content = fs::read_to_string(&output).unwrap();
        assert!(svg_content.contains(r#"opacity="0.5""#));
    }

    #[test]
    fn test_export_svg_ignores_invisible() {
        let temp = tempdir().unwrap();
        let output = temp.path().join("output.svg");
        
        let mut manifest = PifManifest::new(100, 100);
        
        manifest.layers.push(Layer::Vector {
            id: "hidden".to_string(),
            name: "Hidden".to_string(),
            visible: false,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            elements: vec![
                VectorElement::Circle {
                    cx: 50.0,
                    cy: 50.0,
                    r: 25.0,
                    fill: "#hidden".to_string(),
                },
            ],
        });
        
        export_svg(&manifest, &output).unwrap();
        
        let svg_content = fs::read_to_string(&output).unwrap();
        assert!(!svg_content.contains("#hidden"));
    }
}
