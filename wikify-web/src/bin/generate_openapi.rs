//! Generate OpenAPI specification files
//!
//! This binary generates static OpenAPI specification files in JSON and YAML formats.

use std::fs;
use std::path::Path;
use utoipa::OpenApi;
use wikify_web::openapi::{get_openapi_yaml, ApiDoc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating OpenAPI specification files...");

    // Create docs directory if it doesn't exist
    let docs_dir = Path::new("wikify-web/docs");
    if !docs_dir.exists() {
        fs::create_dir_all(docs_dir)?;
    }

    // Generate OpenAPI specification
    let openapi = ApiDoc::openapi();

    // Write JSON file
    let json_content = openapi.to_pretty_json()?;
    let json_path = docs_dir.join("openapi.json");
    fs::write(&json_path, json_content)?;
    println!("✅ Generated: {}", json_path.display());

    // Write YAML file
    let yaml_content = get_openapi_yaml();
    let yaml_path = docs_dir.join("openapi.yaml");
    fs::write(&yaml_path, yaml_content)?;
    println!("✅ Generated: {}", yaml_path.display());

    // Also generate a compact JSON for embedding
    let compact_json = serde_json::to_string(&openapi)?;
    let compact_path = docs_dir.join("openapi.compact.json");
    fs::write(&compact_path, compact_json)?;
    println!("✅ Generated: {}", compact_path.display());

    println!("\n🎉 OpenAPI specification files generated successfully!");
    println!("📁 Files location: {}", docs_dir.display());
    println!("📖 View documentation at: http://localhost:8080/api-docs/docs/");

    Ok(())
}
