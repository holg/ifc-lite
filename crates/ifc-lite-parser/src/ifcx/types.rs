// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! IFC5 (IFCX) type definitions
//!
//! Based on buildingSMART IFC5-development schema.
//! Uses serde for JSON deserialization with minimal allocations.

use serde::Deserialize;
use std::collections::HashMap;

/// Root IFCX file structure
#[derive(Debug, Deserialize)]
pub struct IfcxFile {
    pub header: IfcxHeader,
    #[serde(default)]
    pub imports: Vec<ImportNode>,
    #[serde(default)]
    pub schemas: HashMap<String, serde_json::Value>,
    pub data: Vec<IfcxNode>,
}

/// IFCX file header
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IfcxHeader {
    pub id: String,
    pub ifcx_version: String,
    pub data_version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub timestamp: String,
}

/// Import reference to external schema
#[derive(Debug, Deserialize)]
pub struct ImportNode {
    pub uri: String,
    #[serde(default)]
    pub integrity: Option<String>,
}

/// Raw IFCX node from JSON
#[derive(Debug, Deserialize)]
pub struct IfcxNode {
    pub path: String,
    #[serde(default)]
    pub children: HashMap<String, Option<String>>,
    #[serde(default)]
    pub inherits: HashMap<String, Option<String>>,
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Composed node after flattening ECS structure
#[derive(Debug, Clone)]
pub struct ComposedNode {
    pub path: String,
    pub attributes: rustc_hash::FxHashMap<String, serde_json::Value>,
    pub children: Vec<String>, // Child paths
    pub parent: Option<String>, // Parent path
}

/// Well-known attribute namespace constants
pub mod attr {
    /// IFC classification (bsi::ifc::class)
    pub const CLASS: &str = "bsi::ifc::class";
    /// USD mesh geometry
    pub const MESH: &str = "usd::usdgeom::mesh";
    /// USD transform prefix
    pub const TRANSFORM: &str = "usd::xformop::transform";
    /// USD visibility
    pub const VISIBILITY: &str = "usd::usdgeom::visibility";
    /// Diffuse color
    pub const DIFFUSE_COLOR: &str = "bsi::ifc::presentation::diffuseColor";
    /// Opacity
    pub const OPACITY: &str = "bsi::ifc::presentation::opacity";
    /// Material
    pub const MATERIAL: &str = "bsi::ifc::material";
    /// Property prefix
    pub const PROP_PREFIX: &str = "bsi::ifc::prop::";
}

/// USD mesh data structure
#[derive(Debug, Clone, Default)]
pub struct UsdMesh {
    pub points: Vec<[f64; 3]>,
    pub face_vertex_indices: Vec<u32>,
    pub face_vertex_counts: Option<Vec<u32>>,
    pub normals: Option<Vec<[f64; 3]>>,
}

impl UsdMesh {
    /// Parse from serde_json::Value
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        let obj = value.as_object()?;

        // Parse points: [[x,y,z], ...]
        let points_arr = obj.get("points")?.as_array()?;
        let mut points = Vec::with_capacity(points_arr.len());
        for p in points_arr {
            let arr = p.as_array()?;
            if arr.len() >= 3 {
                points.push([
                    arr[0].as_f64()?,
                    arr[1].as_f64()?,
                    arr[2].as_f64()?,
                ]);
            }
        }

        // Parse faceVertexIndices
        let indices_arr = obj.get("faceVertexIndices")?.as_array()?;
        let mut face_vertex_indices = Vec::with_capacity(indices_arr.len());
        for i in indices_arr {
            face_vertex_indices.push(i.as_u64()? as u32);
        }

        // Parse optional faceVertexCounts
        let face_vertex_counts = obj.get("faceVertexCounts").and_then(|v| {
            let arr = v.as_array()?;
            let mut counts = Vec::with_capacity(arr.len());
            for c in arr {
                counts.push(c.as_u64()? as u32);
            }
            Some(counts)
        });

        // Parse optional normals
        let normals = obj.get("normals").and_then(|v| {
            let arr = v.as_array()?;
            let mut norms = Vec::with_capacity(arr.len());
            for n in arr {
                let narr = n.as_array()?;
                if narr.len() >= 3 {
                    norms.push([
                        narr[0].as_f64()?,
                        narr[1].as_f64()?,
                        narr[2].as_f64()?,
                    ]);
                }
            }
            Some(norms)
        });

        Some(Self {
            points,
            face_vertex_indices,
            face_vertex_counts,
            normals,
        })
    }

    /// Check if mesh is already triangulated
    pub fn is_triangulated(&self) -> bool {
        match &self.face_vertex_counts {
            None => true, // Default is triangles
            Some(counts) => counts.iter().all(|&c| c == 3),
        }
    }

    /// Triangulate mesh (fan triangulation for polygons)
    pub fn triangulate(&self) -> Vec<u32> {
        if self.is_triangulated() {
            return self.face_vertex_indices.clone();
        }

        let counts = self.face_vertex_counts.as_ref().unwrap();
        let mut result = Vec::new();
        let mut idx = 0usize;

        for &count in counts {
            let count = count as usize;
            if count == 3 {
                // Already a triangle
                result.push(self.face_vertex_indices[idx]);
                result.push(self.face_vertex_indices[idx + 1]);
                result.push(self.face_vertex_indices[idx + 2]);
            } else if count > 3 {
                // Fan triangulation: first vertex + consecutive pairs
                let v0 = self.face_vertex_indices[idx];
                for i in 1..(count - 1) {
                    result.push(v0);
                    result.push(self.face_vertex_indices[idx + i]);
                    result.push(self.face_vertex_indices[idx + i + 1]);
                }
            }
            idx += count;
        }

        result
    }
}

/// IFC class reference
#[derive(Debug, Clone)]
pub struct IfcClass {
    pub code: String,
    pub uri: Option<String>,
}

impl IfcClass {
    /// Parse from serde_json::Value
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        let obj = value.as_object()?;
        let code = obj.get("code")?.as_str()?.to_string();
        let uri = obj.get("uri").and_then(|v| v.as_str()).map(String::from);
        Some(Self { code, uri })
    }
}

/// 4x4 transformation matrix (row-major)
#[derive(Debug, Clone, Copy)]
pub struct Transform4x4 {
    pub matrix: [[f64; 4]; 4],
}

impl Default for Transform4x4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform4x4 {
    pub fn identity() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Parse from serde_json::Value (array of arrays)
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        // Can be under "transform" key or directly be array
        let arr = if let Some(obj) = value.as_object() {
            obj.get("transform")?.as_array()?
        } else {
            value.as_array()?
        };

        if arr.len() != 4 {
            return None;
        }

        let mut matrix = [[0.0; 4]; 4];
        for (i, row) in arr.iter().enumerate() {
            let row_arr = row.as_array()?;
            if row_arr.len() != 4 {
                return None;
            }
            for (j, val) in row_arr.iter().enumerate() {
                matrix[i][j] = val.as_f64()?;
            }
        }

        Some(Self { matrix })
    }

    /// Transform a point
    pub fn transform_point(&self, point: [f64; 3]) -> [f64; 3] {
        let m = &self.matrix;
        let w = m[3][0] * point[0] + m[3][1] * point[1] + m[3][2] * point[2] + m[3][3];
        [
            (m[0][0] * point[0] + m[0][1] * point[1] + m[0][2] * point[2] + m[0][3]) / w,
            (m[1][0] * point[0] + m[1][1] * point[1] + m[1][2] * point[2] + m[1][3]) / w,
            (m[2][0] * point[0] + m[2][1] * point[1] + m[2][2] * point[2] + m[2][3]) / w,
        ]
    }

    /// Multiply two matrices
    pub fn multiply(&self, other: &Self) -> Self {
        let mut result = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += self.matrix[i][k] * other.matrix[k][j];
                }
            }
        }
        Self { matrix: result }
    }
}
