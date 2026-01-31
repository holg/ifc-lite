// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Tests for IFC5 (IFCX) parser

use ifc_lite_model::{GeometrySource, IfcModel};
use ifc_lite_parser::{is_ifcx_format, parse_auto, IfcxGeometry, IfcxModel};
use std::fs;

const HELLO_WALL_PATH: &str = "../../tests/models/ifc5/Hello_Wall_hello-wall.ifcx";

#[test]
fn test_format_detection() {
    let ifcx_content = r#"{"header": {"id": "test"}}"#;
    let step_content = "ISO-10303-21;\nHEADER;";

    assert!(is_ifcx_format(ifcx_content));
    assert!(!is_ifcx_format(step_content));
}

#[test]
fn test_parse_hello_wall() {
    let content = match fs::read_to_string(HELLO_WALL_PATH) {
        Ok(c) => c,
        Err(_) => {
            println!("Skipping test - file not found: {}", HELLO_WALL_PATH);
            return;
        }
    };

    assert!(is_ifcx_format(&content), "Should detect IFCX format");

    let model = IfcxModel::parse(&content).expect("Failed to parse IFCX");

    // Check metadata
    assert!(
        model.metadata().schema_version.contains("IFC5"),
        "Should be IFC5 schema"
    );

    // Check entities were parsed
    let entity_count = model.resolver().entity_count();
    assert!(entity_count > 0, "Should have parsed entities");
    println!("Parsed {} entities", entity_count);

    // Check spatial tree exists
    let tree = model.spatial().spatial_tree();
    assert!(tree.is_some(), "Should have spatial tree");

    if let Some(root) = tree {
        println!("Root: {} ({})", root.name, root.entity_type);
        println!("Children: {}", root.children.len());
    }

    // Check we can find walls
    let walls = model.resolver().find_by_type_name("IfcWall");
    println!("Found {} walls", walls.len());
}

#[test]
fn test_parse_auto_detects_format() {
    let content = match fs::read_to_string(HELLO_WALL_PATH) {
        Ok(c) => c,
        Err(_) => return, // Skip if file not found
    };

    let model = parse_auto(&content).expect("parse_auto should work");
    assert!(model.metadata().schema_version.contains("IFC5"));
}

#[test]
fn test_properties() {
    let content = match fs::read_to_string(HELLO_WALL_PATH) {
        Ok(c) => c,
        Err(_) => return,
    };

    let model = IfcxModel::parse(&content).expect("Failed to parse");

    // Check properties for some entity
    let ids = model.resolver().all_ids();
    for id in ids.iter().take(5) {
        let psets = model.properties().property_sets(*id);
        if !psets.is_empty() {
            println!("Entity {} has {} property sets", id, psets.len());
            for pset in &psets {
                println!("  {}: {} properties", pset.name, pset.properties.len());
            }
            break;
        }
    }
}

#[test]
fn test_geometry_extraction() {
    let content = match fs::read_to_string(HELLO_WALL_PATH) {
        Ok(c) => c,
        Err(_) => return,
    };

    let model = std::sync::Arc::new(IfcxModel::parse(&content).expect("Failed to parse"));
    let geometry = IfcxGeometry::new(model.clone());

    // Check entities with geometry
    let geom_entities = geometry.entities_with_geometry();
    println!("Entities with geometry: {}", geom_entities.len());

    // Try to extract geometry
    let mut total_triangles = 0;
    for id in geom_entities.iter().take(5) {
        if let Some(geom) = geometry.get_geometry(*id) {
            println!(
                "Entity {}: {} triangles, color: {:?}",
                id,
                geom.triangle_count(),
                geom.color
            );
            total_triangles += geom.triangle_count();
        }
    }

    println!("Total triangles (first 5): {}", total_triangles);
}
