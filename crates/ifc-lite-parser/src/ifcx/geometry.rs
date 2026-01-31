// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! IFC5 (IFCX) geometry extraction
//!
//! Extracts pre-tessellated USD mesh geometry from IFCX nodes.
//! Unlike IFC4 which requires complex geometry processing,
//! IFC5 provides ready-to-use triangulated meshes.

use super::model::IfcxModel;
use super::types::{attr, Transform4x4, UsdMesh};
use ifc_lite_model::{get_default_color, EntityGeometry, EntityId, GeometrySource, IfcModel, MeshData};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Geometry source for IFCX models
pub struct IfcxGeometry {
    /// Reference to the model
    model: Arc<IfcxModel>,
    /// Cached geometry by entity ID (for future use)
    #[allow(dead_code)]
    cache: FxHashMap<EntityId, Option<EntityGeometry>>,
    /// Entity IDs with geometry
    entities_with_geom: Vec<EntityId>,
}

impl IfcxGeometry {
    /// Create geometry source from IFCX model
    pub fn new(model: Arc<IfcxModel>) -> Self {
        // Find all entities with geometry
        let mut entities_with_geom = Vec::new();

        for id in model.resolver().all_ids() {
            if let Some(node) = model.node(id) {
                if node.attributes.contains_key(attr::MESH) {
                    entities_with_geom.push(id);
                }
            }
        }

        Self {
            model,
            cache: FxHashMap::default(),
            entities_with_geom,
        }
    }

    /// Extract geometry for a single entity
    fn extract_geometry(&self, id: EntityId) -> Option<EntityGeometry> {
        let node = self.model.node(id)?;

        // Get USD mesh data
        let mesh_value = node.attributes.get(attr::MESH)?;
        let usd_mesh = UsdMesh::from_value(mesh_value)?;

        // Convert to MeshData
        let mesh_data = usd_mesh_to_mesh_data(&usd_mesh)?;

        // Get transform (if any)
        let transform = node
            .attributes
            .get(attr::TRANSFORM)
            .and_then(Transform4x4::from_value)
            .unwrap_or_default();

        // Get color from presentation attributes or default by type
        let color = extract_color(node, &self.model);

        // Convert transform to column-major f32 array
        let transform_array = transform_to_array(&transform);

        Some(EntityGeometry::new(
            Arc::new(mesh_data),
            color,
            transform_array,
        ))
    }
}

impl GeometrySource for IfcxGeometry {
    fn entities_with_geometry(&self) -> Vec<EntityId> {
        self.entities_with_geom.clone()
    }

    fn has_geometry(&self, id: EntityId) -> bool {
        if let Some(node) = self.model.node(id) {
            node.attributes.contains_key(attr::MESH)
        } else {
            false
        }
    }

    fn get_geometry(&self, id: EntityId) -> Option<EntityGeometry> {
        // Note: In a real implementation we'd use interior mutability for caching
        // For now, just extract directly
        self.extract_geometry(id)
    }
}

/// Convert USD mesh to MeshData format
fn usd_mesh_to_mesh_data(usd: &UsdMesh) -> Option<MeshData> {
    if usd.points.is_empty() {
        return None;
    }

    // Get triangulated indices
    let indices = usd.triangulate();
    if indices.is_empty() {
        return None;
    }

    // Convert points to flat f32 array
    let mut positions = Vec::with_capacity(usd.points.len() * 3);
    for p in &usd.points {
        positions.push(p[0] as f32);
        positions.push(p[1] as f32);
        positions.push(p[2] as f32);
    }

    // Compute or use provided normals
    let normals = if let Some(ref usd_normals) = usd.normals {
        // Use provided normals
        let mut normals = Vec::with_capacity(usd_normals.len() * 3);
        for n in usd_normals {
            normals.push(n[0] as f32);
            normals.push(n[1] as f32);
            normals.push(n[2] as f32);
        }
        normals
    } else {
        // Compute normals from triangles
        compute_normals(&positions, &indices)
    };

    Some(MeshData {
        positions,
        normals,
        indices,
    })
}

/// Compute flat normals for triangles
fn compute_normals(positions: &[f32], indices: &[u32]) -> Vec<f32> {
    let vertex_count = positions.len() / 3;
    let mut normals = vec![0.0f32; vertex_count * 3];
    let mut counts = vec![0u32; vertex_count];

    // Accumulate face normals for each vertex
    for tri in indices.chunks(3) {
        if tri.len() < 3 {
            continue;
        }

        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;

        if i0 * 3 + 2 >= positions.len()
            || i1 * 3 + 2 >= positions.len()
            || i2 * 3 + 2 >= positions.len()
        {
            continue;
        }

        // Get vertices
        let v0 = [
            positions[i0 * 3],
            positions[i0 * 3 + 1],
            positions[i0 * 3 + 2],
        ];
        let v1 = [
            positions[i1 * 3],
            positions[i1 * 3 + 1],
            positions[i1 * 3 + 2],
        ];
        let v2 = [
            positions[i2 * 3],
            positions[i2 * 3 + 1],
            positions[i2 * 3 + 2],
        ];

        // Compute edges
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        // Cross product
        let nx = e1[1] * e2[2] - e1[2] * e2[1];
        let ny = e1[2] * e2[0] - e1[0] * e2[2];
        let nz = e1[0] * e2[1] - e1[1] * e2[0];

        // Accumulate to each vertex
        for &idx in &[i0, i1, i2] {
            normals[idx * 3] += nx;
            normals[idx * 3 + 1] += ny;
            normals[idx * 3 + 2] += nz;
            counts[idx] += 1;
        }
    }

    // Normalize
    for i in 0..vertex_count {
        if counts[i] > 0 {
            let nx = normals[i * 3];
            let ny = normals[i * 3 + 1];
            let nz = normals[i * 3 + 2];
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if len > 1e-6 {
                normals[i * 3] = nx / len;
                normals[i * 3 + 1] = ny / len;
                normals[i * 3 + 2] = nz / len;
            } else {
                // Default up normal
                normals[i * 3] = 0.0;
                normals[i * 3 + 1] = 1.0;
                normals[i * 3 + 2] = 0.0;
            }
        }
    }

    normals
}

/// Extract color from node attributes
fn extract_color(
    node: &super::types::ComposedNode,
    model: &IfcxModel,
) -> [f32; 4] {
    // Try diffuse color attribute
    if let Some(color_val) = node.attributes.get(attr::DIFFUSE_COLOR) {
        if let Some(arr) = color_val.as_array() {
            if arr.len() >= 3 {
                let r = arr[0].as_f64().unwrap_or(0.7) as f32;
                let g = arr[1].as_f64().unwrap_or(0.7) as f32;
                let b = arr[2].as_f64().unwrap_or(0.7) as f32;

                // Get opacity
                let a = node
                    .attributes
                    .get(attr::OPACITY)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;

                return [r, g, b, a];
            }
        }
    }

    // Fall back to default color based on type
    if let Some(entity) = model.resolver().get(model.id_for_path(&node.path).unwrap_or_default()) {
        return get_default_color(&entity.ifc_type);
    }

    // Ultimate fallback
    [0.7, 0.7, 0.7, 1.0]
}

/// Convert Transform4x4 to column-major f32 array
fn transform_to_array(transform: &Transform4x4) -> [f32; 16] {
    let m = &transform.matrix;
    // Column-major order for OpenGL/GPU
    [
        m[0][0] as f32,
        m[1][0] as f32,
        m[2][0] as f32,
        m[3][0] as f32,
        m[0][1] as f32,
        m[1][1] as f32,
        m[2][1] as f32,
        m[3][1] as f32,
        m[0][2] as f32,
        m[1][2] as f32,
        m[2][2] as f32,
        m[3][2] as f32,
        m[0][3] as f32,
        m[1][3] as f32,
        m[2][3] as f32,
        m[3][3] as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usd_mesh_conversion() {
        let usd = UsdMesh {
            points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            face_vertex_indices: vec![0, 1, 2],
            face_vertex_counts: None,
            normals: None,
        };

        let mesh = usd_mesh_to_mesh_data(&usd).unwrap();

        assert_eq!(mesh.positions.len(), 9); // 3 vertices * 3 components
        assert_eq!(mesh.indices.len(), 3); // 1 triangle
        assert_eq!(mesh.normals.len(), 9); // 3 vertices * 3 components
    }

    #[test]
    fn test_triangulation() {
        // Quad face
        let usd = UsdMesh {
            points: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            face_vertex_indices: vec![0, 1, 2, 3],
            face_vertex_counts: Some(vec![4]), // One quad
            normals: None,
        };

        let indices = usd.triangulate();
        assert_eq!(indices.len(), 6); // 2 triangles * 3 vertices
    }
}
