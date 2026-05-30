//! Example: Create a vector-based logo and export to SVG
//!
//! Run with: cargo run --example vector_logo

use pulsar_image_format::{PifAssetManager, SaveMode};
use pulsar_image_format::model::{Layer, VectorElement};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating a new 512x512 vector logo...");
    
    let mut manager = PifAssetManager::create_new(
        "logo.pif",
        512,
        512,
        SaveMode::Directory,
    )?;

    println!("Adding vector shapes...");
    manager.manifest_mut().layers.push(Layer::Vector {
        id: "logo_bg".to_string(),
        name: "Background".to_string(),
        visible: true,
        opacity: 0.9,
        blend_mode: "normal".to_string(),
        elements: vec![
            VectorElement::Circle {
                cx: 256.0,
                cy: 256.0,
                r: 200.0,
                fill: "#4A90E2".to_string(),
            },
        ],
    });

    manager.manifest_mut().layers.push(Layer::Vector {
        id: "logo_icon".to_string(),
        name: "Icon".to_string(),
        visible: true,
        opacity: 1.0,
        blend_mode: "normal".to_string(),
        elements: vec![
            VectorElement::Rect {
                x: 196.0,
                y: 196.0,
                w: 120.0,
                h: 120.0,
                fill: "#FFFFFF".to_string(),
            },
        ],
    });

    println!("Exporting to SVG...");
    manager.bake("logo_output.svg", "svg")?;

    println!("✓ Logo created:");
    println!("  - PIF source: logo.pif/");
    println!("  - SVG export: logo_output.svg");
    println!("  - Layers: 2 vector layers");
    
    Ok(())
}
