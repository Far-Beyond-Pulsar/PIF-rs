//! Example: Create a simple PIF image and paint some pixels
//!
//! Run with: cargo run --example simple_painting

use pulsar_image_format::{PifAssetManager, SaveMode};
use pulsar_image_format::model::Layer;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating a new 1024x768 PIF image...");
    
    let mut manager = PifAssetManager::create_new(
        "example_output.pif",
        1024,
        768,
        SaveMode::Directory,
    )?;

    println!("Adding a background layer...");
    manager.manifest_mut().layers.push(Layer::Raster {
        id: "background".to_string(),
        name: "Background".to_string(),
        visible: true,
        opacity: 1.0,
        blend_mode: "normal".to_string(),
        tile_size: 256,
        tiles: HashMap::new(),
    });

    println!("Loading tile (0, 0)...");
    let mut pixels = manager.load_raster_tile("background", 0, 0)?;

    println!("Painting a gradient in the top-left corner...");
    for y in 0..256 {
        for x in 0..256 {
            let idx = (y * 256 + x) * 4;
            pixels[idx] = x as u8;     // Red channel
            pixels[idx + 1] = y as u8; // Green channel
            pixels[idx + 2] = 128;     // Blue channel
            pixels[idx + 3] = 255;     // Alpha channel
        }
    }

    println!("Committing changes...");
    let mut changes = HashMap::new();
    changes.insert(("background".to_string(), 0, 0), pixels);
    manager.commit_changes(changes)?;

    println!("✓ Image saved to example_output.pif/");
    println!("  - Canvas: 1024×768");
    println!("  - Layers: 1 (Background)");
    println!("  - Tiles: 1 (256×256 gradient)");
    
    Ok(())
}
