//! IFC-Lite FFI - UniFFI bindings for Swift (iOS/macOS) and Kotlin (Android)
//!
//! This crate provides cross-platform bindings to the IFC-Lite library,
//! allowing native iOS, macOS, and Android apps to load and interact with IFC files.

use ifc_lite_core::DecodedEntity;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;

/// Helper to extract entity refs from a list attribute
fn get_ref_list(entity: &DecodedEntity, index: usize) -> Option<Vec<u32>> {
    entity
        .get_list(index)
        .map(|list| list.iter().filter_map(|v| v.as_entity_ref()).collect())
}

// Export UniFFI scaffolding
uniffi::setup_scaffolding!();

/// Library version
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the library (call once at app startup)
#[uniffi::export]
pub fn init_library() {
    // Initialize any global state if needed
}

/// Get library version
#[uniffi::export]
pub fn get_version() -> String {
    VERSION.to_string()
}

/// Error type for FFI operations
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum IfcError {
    #[error("Parse error: {msg}")]
    ParseError { msg: String },
    #[error("Geometry error: {msg}")]
    GeometryError { msg: String },
    #[error("IO error: {msg}")]
    IoError { msg: String },
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Scene not loaded")]
    NotLoaded,
}

impl From<std::io::Error> for IfcError {
    fn from(e: std::io::Error) -> Self {
        IfcError::IoError { msg: e.to_string() }
    }
}

/// Entity information
#[derive(Debug, Clone, uniffi::Record)]
pub struct EntityInfo {
    pub id: u64,
    pub entity_type: String,
    pub name: Option<String>,
    pub global_id: Option<String>,
    pub storey: Option<String>,
    pub storey_elevation: Option<f32>,
}

/// Mesh data for rendering (per-entity, use for individual mesh access)
#[derive(Debug, Clone, uniffi::Record)]
pub struct MeshData {
    pub entity_id: u64,
    pub entity_type: String,
    pub name: Option<String>,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub color: Vec<f32>,     // RGBA
    pub transform: Vec<f32>, // 4x4 matrix
}

/// Batched mesh data for efficient rendering
/// All vertices are pre-transformed to world space and combined into single buffers.
/// Use this for maximum rendering performance (2 draw calls instead of N).
#[derive(Debug, Clone, uniffi::Record)]
pub struct BatchedMeshData {
    /// Interleaved vertex data: [x, y, z, nx, ny, nz, r, g, b, a, ...] (10 floats per vertex)
    pub vertices: Vec<f32>,
    /// Triangle indices
    pub indices: Vec<u32>,
    /// Whether this batch contains transparent geometry
    pub is_transparent: bool,
    /// Number of vertices
    pub vertex_count: u32,
    /// Number of triangles
    pub triangle_count: u32,
}

/// Scene bounds (AABB)
#[derive(Debug, Clone, uniffi::Record)]
pub struct SceneBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
}

/// Spatial hierarchy node
#[derive(Debug, Clone, uniffi::Record)]
pub struct SpatialNode {
    pub id: u64,
    pub node_type: String,
    pub name: String,
    pub entity_type: String,
    pub elevation: Option<f32>,
    pub has_geometry: bool,
    pub children: Vec<SpatialNode>,
}

/// Property set
#[derive(Debug, Clone, uniffi::Record)]
pub struct PropertySet {
    pub name: String,
    pub properties: Vec<PropertyValue>,
}

/// Property value
#[derive(Debug, Clone, uniffi::Record)]
pub struct PropertyValue {
    pub name: String,
    pub value: String,
    pub unit: Option<String>,
}

/// Load result
#[derive(Debug, Clone, uniffi::Record)]
pub struct LoadResult {
    pub meshes: Vec<MeshData>,
    pub entities: Vec<EntityInfo>,
    pub spatial_tree: Option<SpatialNode>,
    pub bounds: Option<SceneBounds>,
    pub load_time_ms: u64,
}

/// Camera state
#[derive(Debug, Clone, uniffi::Record)]
pub struct CameraState {
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            azimuth: 0.785,   // 45 degrees
            elevation: 0.615, // ~35 degrees (isometric)
            distance: 100.0,
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,
        }
    }
}

/// Selection state
#[derive(Debug, Clone, uniffi::Record)]
pub struct SelectionState {
    pub selected_ids: Vec<u64>,
    pub hovered_id: Option<u64>,
}

/// Visibility state
#[derive(Debug, Clone, uniffi::Record)]
pub struct VisibilityState {
    pub hidden_ids: Vec<u64>,
    pub isolated_ids: Option<Vec<u64>>,
    pub storey_filter: Option<String>,
}

/// Section plane
#[derive(Debug, Clone, uniffi::Record)]
pub struct SectionPlane {
    pub enabled: bool,
    pub origin_x: f32,
    pub origin_y: f32,
    pub origin_z: f32,
    pub normal_x: f32,
    pub normal_y: f32,
    pub normal_z: f32,
}

impl Default for SectionPlane {
    fn default() -> Self {
        Self {
            enabled: false,
            origin_x: 0.0,
            origin_y: 0.0,
            origin_z: 0.0,
            normal_x: 0.0,
            normal_y: 1.0,
            normal_z: 0.0,
        }
    }
}

/// Internal scene data
#[derive(Default)]
struct SceneData {
    meshes: Vec<MeshData>,
    entities: Vec<EntityInfo>,
    spatial_tree: Option<SpatialNode>,
    bounds: Option<SceneBounds>,

    // State
    selected_ids: HashSet<u64>,
    hovered_id: Option<u64>,
    hidden_ids: HashSet<u64>,
    isolated_ids: Option<HashSet<u64>>,
    storey_filter: Option<String>,
    camera: CameraState,
    section_plane: SectionPlane,

    // Original content for property lookups
    #[allow(dead_code)]
    content: Option<String>,
}

/// Main IFC Scene interface - thread-safe
#[derive(uniffi::Object)]
pub struct IfcScene {
    data: Arc<RwLock<SceneData>>,
}

#[uniffi::export]
impl IfcScene {
    /// Create a new empty scene
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(SceneData::default())),
        }
    }

    /// Load IFC from file path (native only)
    pub fn load_file(&self, path: String) -> Result<LoadResult, IfcError> {
        let content = std::fs::read_to_string(&path)?;
        self.load_string(content)
    }

    /// Load IFC from bytes
    pub fn load_bytes(&self, data: Vec<u8>) -> Result<LoadResult, IfcError> {
        let content = String::from_utf8(data).map_err(|e| IfcError::ParseError {
            msg: format!("Invalid UTF-8: {}", e),
        })?;
        self.load_string(content)
    }

    /// Load IFC from string content
    pub fn load_string(&self, content: String) -> Result<LoadResult, IfcError> {
        let start = std::time::Instant::now();

        // Parse and process the IFC content
        let (meshes, entities, spatial_tree, bounds) = process_ifc_content(&content)?;

        let load_time_ms = start.elapsed().as_millis() as u64;

        // Update scene data
        {
            let mut data = self.data.write();
            data.meshes = meshes.clone();
            data.entities = entities.clone();
            data.spatial_tree = spatial_tree.clone();
            data.bounds = bounds.clone();
            data.content = Some(content);

            // Reset state
            data.selected_ids.clear();
            data.hovered_id = None;
            data.hidden_ids.clear();
            data.isolated_ids = None;
            data.storey_filter = None;
        }

        Ok(LoadResult {
            meshes,
            entities,
            spatial_tree,
            bounds,
            load_time_ms,
        })
    }

    /// Check if scene has data
    pub fn is_loaded(&self) -> bool {
        let data = self.data.read();
        !data.entities.is_empty()
    }

    /// Get all entities
    pub fn get_entities(&self) -> Vec<EntityInfo> {
        self.data.read().entities.clone()
    }

    /// Get entity by ID
    pub fn get_entity(&self, id: u64) -> Option<EntityInfo> {
        self.data
            .read()
            .entities
            .iter()
            .find(|e| e.id == id)
            .cloned()
    }

    /// Get spatial hierarchy tree
    pub fn get_spatial_tree(&self) -> Option<SpatialNode> {
        self.data.read().spatial_tree.clone()
    }

    /// Get scene bounds
    pub fn get_bounds(&self) -> Option<SceneBounds> {
        self.data.read().bounds.clone()
    }

    /// Get all meshes (per-entity, slower rendering)
    pub fn get_meshes(&self) -> Vec<MeshData> {
        self.data.read().meshes.clone()
    }

    /// Get mesh for specific entity
    pub fn get_mesh(&self, entity_id: u64) -> Option<MeshData> {
        self.data
            .read()
            .meshes
            .iter()
            .find(|m| m.entity_id == entity_id)
            .cloned()
    }

    /// Get batched meshes for efficient rendering
    /// Returns 2 batches: opaque geometry and transparent geometry.
    /// All vertices are pre-transformed to world space with vertex colors.
    /// Use this for maximum rendering performance.
    pub fn get_batched_meshes(&self) -> Vec<BatchedMeshData> {
        let data = self.data.read();
        let meshes = &data.meshes;

        if meshes.is_empty() {
            return Vec::new();
        }

        // Separate opaque and transparent
        let mut opaque_vertices: Vec<f32> = Vec::new();
        let mut opaque_indices: Vec<u32> = Vec::new();
        let mut transparent_vertices: Vec<f32> = Vec::new();
        let mut transparent_indices: Vec<u32> = Vec::new();

        for mesh in meshes {
            let is_transparent = mesh.color.len() >= 4 && mesh.color[3] < 1.0;
            let (vertices, indices) = if is_transparent {
                (&mut transparent_vertices, &mut transparent_indices)
            } else {
                (&mut opaque_vertices, &mut opaque_indices)
            };

            let vertex_offset = (vertices.len() / 10) as u32;
            let vertex_count = mesh.positions.len() / 3;

            // Get transform matrix
            let transform = if mesh.transform.len() == 16 {
                nalgebra::Matrix4::from_column_slice(&mesh.transform)
            } else {
                nalgebra::Matrix4::identity()
            };

            // Get color (RGBA)
            let color = if mesh.color.len() >= 4 {
                [mesh.color[0], mesh.color[1], mesh.color[2], mesh.color[3]]
            } else if mesh.color.len() >= 3 {
                [mesh.color[0], mesh.color[1], mesh.color[2], 1.0]
            } else {
                [0.8, 0.8, 0.8, 1.0]
            };

            // Add vertices with transform applied
            for i in 0..vertex_count {
                let idx = i * 3;

                // Position (IFC Z-up to Y-up)
                let local_pos = nalgebra::Point3::new(
                    mesh.positions[idx],
                    mesh.positions.get(idx + 2).copied().unwrap_or(0.0), // Z -> Y
                    -mesh.positions.get(idx + 1).copied().unwrap_or(0.0), // -Y -> Z
                );
                let world_pos = transform.transform_point(&local_pos);

                // Normal (IFC Z-up to Y-up)
                let local_normal = if mesh.normals.len() > idx + 2 {
                    nalgebra::Vector3::new(
                        mesh.normals[idx],
                        mesh.normals[idx + 2],  // Z -> Y
                        -mesh.normals[idx + 1], // -Y -> Z
                    )
                } else {
                    nalgebra::Vector3::new(0.0, 1.0, 0.0)
                };
                let world_normal = transform.fixed_view::<3, 3>(0, 0).into_owned() * local_normal;

                // Interleaved: [x, y, z, nx, ny, nz, r, g, b, a]
                vertices.push(world_pos.x);
                vertices.push(world_pos.y);
                vertices.push(world_pos.z);
                vertices.push(world_normal.x);
                vertices.push(world_normal.y);
                vertices.push(world_normal.z);
                vertices.push(color[0]);
                vertices.push(color[1]);
                vertices.push(color[2]);
                vertices.push(color[3]);
            }

            // Add indices with offset
            for idx in &mesh.indices {
                indices.push(idx + vertex_offset);
            }
        }

        let mut result = Vec::new();

        if !opaque_vertices.is_empty() {
            result.push(BatchedMeshData {
                vertex_count: (opaque_vertices.len() / 10) as u32,
                triangle_count: (opaque_indices.len() / 3) as u32,
                vertices: opaque_vertices,
                indices: opaque_indices,
                is_transparent: false,
            });
        }

        if !transparent_vertices.is_empty() {
            result.push(BatchedMeshData {
                vertex_count: (transparent_vertices.len() / 10) as u32,
                triangle_count: (transparent_indices.len() / 3) as u32,
                vertices: transparent_vertices,
                indices: transparent_indices,
                is_transparent: true,
            });
        }

        result
    }

    /// Get properties for entity
    pub fn get_properties(&self, entity_id: u64) -> Vec<PropertySet> {
        let data = self.data.read();
        let content = match &data.content {
            Some(c) => c,
            None => return Vec::new(),
        };

        extract_properties(content, entity_id as u32)
    }

    // Selection methods
    pub fn select(&self, entity_id: u64) {
        let mut data = self.data.write();
        data.selected_ids.clear();
        data.selected_ids.insert(entity_id);
    }

    pub fn add_to_selection(&self, entity_id: u64) {
        self.data.write().selected_ids.insert(entity_id);
    }

    pub fn remove_from_selection(&self, entity_id: u64) {
        self.data.write().selected_ids.remove(&entity_id);
    }

    pub fn clear_selection(&self) {
        self.data.write().selected_ids.clear();
    }

    pub fn toggle_selection(&self, entity_id: u64) {
        let mut data = self.data.write();
        if data.selected_ids.contains(&entity_id) {
            data.selected_ids.remove(&entity_id);
        } else {
            data.selected_ids.insert(entity_id);
        }
    }

    pub fn get_selection(&self) -> SelectionState {
        let data = self.data.read();
        SelectionState {
            selected_ids: data.selected_ids.iter().copied().collect(),
            hovered_id: data.hovered_id,
        }
    }

    // Visibility methods
    pub fn hide_entity(&self, entity_id: u64) {
        self.data.write().hidden_ids.insert(entity_id);
    }

    pub fn show_entity(&self, entity_id: u64) {
        self.data.write().hidden_ids.remove(&entity_id);
    }

    pub fn isolate_entity(&self, entity_id: u64) {
        let mut data = self.data.write();
        let mut isolated = HashSet::new();
        isolated.insert(entity_id);
        data.isolated_ids = Some(isolated);
    }

    pub fn isolate_entities(&self, entity_ids: Vec<u64>) {
        let mut data = self.data.write();
        data.isolated_ids = Some(entity_ids.into_iter().collect());
    }

    pub fn show_all(&self) {
        let mut data = self.data.write();
        data.hidden_ids.clear();
        data.isolated_ids = None;
    }

    pub fn set_storey_filter(&self, storey: Option<String>) {
        self.data.write().storey_filter = storey;
    }

    pub fn get_visibility(&self) -> VisibilityState {
        let data = self.data.read();
        VisibilityState {
            hidden_ids: data.hidden_ids.iter().copied().collect(),
            isolated_ids: data
                .isolated_ids
                .as_ref()
                .map(|s| s.iter().copied().collect()),
            storey_filter: data.storey_filter.clone(),
        }
    }

    pub fn is_entity_visible(&self, entity_id: u64) -> bool {
        let data = self.data.read();

        // Hidden check
        if data.hidden_ids.contains(&entity_id) {
            return false;
        }

        // Isolated check
        if let Some(ref isolated) = data.isolated_ids {
            if !isolated.contains(&entity_id) {
                return false;
            }
        }

        // Storey filter check
        if let Some(ref storey_filter) = data.storey_filter {
            if let Some(entity) = data.entities.iter().find(|e| e.id == entity_id) {
                if entity.storey.as_ref() != Some(storey_filter) {
                    return false;
                }
            }
        }

        true
    }

    pub fn get_visible_count(&self) -> u32 {
        let data = self.data.read();
        data.entities
            .iter()
            .filter(|e| {
                !data.hidden_ids.contains(&e.id)
                    && data
                        .isolated_ids
                        .as_ref()
                        .is_none_or(|iso| iso.contains(&e.id))
                    && data
                        .storey_filter
                        .as_ref()
                        .is_none_or(|sf| e.storey.as_ref() == Some(sf))
            })
            .count() as u32
    }

    // Camera
    pub fn set_camera_state(&self, state: CameraState) {
        self.data.write().camera = state;
    }

    pub fn get_camera_state(&self) -> CameraState {
        self.data.read().camera.clone()
    }

    // Section plane
    pub fn set_section_plane(&self, plane: SectionPlane) {
        self.data.write().section_plane = plane;
    }

    pub fn get_section_plane(&self) -> SectionPlane {
        self.data.read().section_plane.clone()
    }

    /// Clear all scene data
    pub fn clear(&self) {
        *self.data.write() = SceneData::default();
    }
}

impl Default for IfcScene {
    fn default() -> Self {
        Self::new()
    }
}

/// Spatial structure entity info (internal)
struct SpatialInfo {
    name: String,
    entity_type: String,
    elevation: Option<f32>,
}

/// Result type for processed IFC content
type ProcessedIfcContent = (
    Vec<MeshData>,
    Vec<EntityInfo>,
    Option<SpatialNode>,
    Option<SceneBounds>,
);

/// Process IFC content and extract meshes, entities, and spatial tree
fn process_ifc_content(content: &str) -> Result<ProcessedIfcContent, IfcError> {
    use ifc_lite_core::{build_entity_index, EntityDecoder, EntityScanner};
    use ifc_lite_geometry::GeometryRouter;
    use std::collections::HashMap;

    // Build entity index for O(1) lookups
    let index = build_entity_index(content);

    // Create decoder with pre-built index
    let mut decoder = EntityDecoder::with_index(content, index);

    // ============ First Pass: Collect spatial structure ============
    // Spatial entities: Project, Site, Building, Storey, Space
    let mut spatial_entities: HashMap<u32, SpatialInfo> = HashMap::new();
    // IfcRelAggregates: parent -> children (for Project->Site->Building->Storey)
    let mut aggregates: HashMap<u32, Vec<u32>> = HashMap::new();
    // IfcRelContainedInSpatialStructure: spatial_element -> contained elements
    let mut contained_in: HashMap<u32, Vec<u32>> = HashMap::new();
    // Element to storey mapping
    let mut element_to_storey: HashMap<u32, u32> = HashMap::new();
    // Track project ID for unit extraction
    let mut project_id: Option<u32> = None;

    // Use EntityScanner for first pass to handle multiline entities
    let mut first_scanner = EntityScanner::new(content);
    let mut rel_count = 0;
    let mut entity_count = 0;
    while let Some((id, type_name, _, _)) = first_scanner.next_entity() {
        entity_count += 1;
        let type_upper = type_name.to_uppercase();

        // Debug: count any relationship entities
        if type_upper.contains("REL") {
            rel_count += 1;
            if rel_count <= 5 {
                eprintln!(
                    "DEBUG FFI: Found relationship entity #{}: {}",
                    id, type_name
                );
            }
        }

        // Debug: check for specific IDs we know are IFCRELAGGREGATES
        if id == 38331 || id == 38275 || id == 38276 {
            eprintln!(
                "DEBUG FFI: Entity #{} has type '{}' (len={}, bytes={:?})",
                id,
                type_name,
                type_name.len(),
                type_name.as_bytes()
            );
        }

        // Parse spatial structure entities
        match type_upper.as_str() {
            "IFCPROJECT" => {
                project_id = Some(id);
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Project".to_string());
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            name,
                            entity_type: type_name.to_string(),
                            elevation: None,
                        },
                    );
                }
            }
            "IFCSITE" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Site".to_string());
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            name,
                            entity_type: type_name.to_string(),
                            elevation: None,
                        },
                    );
                }
            }
            "IFCBUILDING" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Building".to_string());
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            name,
                            entity_type: type_name.to_string(),
                            elevation: None,
                        },
                    );
                }
            }
            "IFCBUILDINGSTOREY" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Storey #{}", id));
                    let elevation = entity.get_float(9).map(|e| e as f32);
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            name,
                            entity_type: type_name.to_string(),
                            elevation,
                        },
                    );
                }
            }
            "IFCSPACE" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Space #{}", id));
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            name,
                            entity_type: type_name.to_string(),
                            elevation: None,
                        },
                    );
                }
            }
            // Parse IfcRelAggregates for parent-child relationships
            // Structure: (GlobalId, OwnerHistory, Name, Description, RelatingObject, RelatedObjects)
            "IFCRELAGGREGATES" => {
                eprintln!("DEBUG FFI: Found IFCRELAGGREGATES #{}", id);
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let parent_id = entity.get_ref(4);
                    let children = get_ref_list(&entity, 5);
                    eprintln!(
                        "DEBUG FFI:   parent={:?}, children={:?}",
                        parent_id,
                        children.as_ref().map(|c| c.len())
                    );
                    if let (Some(parent_id), Some(children)) = (parent_id, children) {
                        aggregates.entry(parent_id).or_default().extend(children);
                    }
                }
            }
            // Also check IfcRelDecomposes (parent class of IfcRelAggregates in IFC2x3)
            "IFCRELDECOMPOSES" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let parent_id = entity.get_ref(4);
                    let children = get_ref_list(&entity, 5);
                    if let (Some(parent_id), Some(children)) = (parent_id, children) {
                        aggregates.entry(parent_id).or_default().extend(children);
                    }
                }
            }
            // IfcRelNests can also define hierarchy
            "IFCRELNESTS" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let parent_id = entity.get_ref(4);
                    let children = get_ref_list(&entity, 5);
                    if let (Some(parent_id), Some(children)) = (parent_id, children) {
                        aggregates.entry(parent_id).or_default().extend(children);
                    }
                }
            }
            // Parse IfcRelContainedInSpatialStructure
            // Structure: (GlobalId, OwnerHistory, Name, Description, RelatedElements, RelatingStructure)
            "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                eprintln!("DEBUG FFI: Found IFCRELCONTAINEDINSPATIALSTRUCTURE #{}", id);
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let structure_id = entity.get_ref(5);
                    let elements = get_ref_list(&entity, 4);
                    eprintln!(
                        "DEBUG FFI:   structure_id={:?}, elements={:?}",
                        structure_id,
                        elements.as_ref().map(|e| e.len())
                    );
                    if let Some(structure_id) = structure_id {
                        if let Some(elements) = elements {
                            contained_in
                                .entry(structure_id)
                                .or_default()
                                .extend(elements.clone());
                            // Also track element -> storey for flat view
                            for elem_id in elements {
                                element_to_storey.insert(elem_id, structure_id);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Extract unit scale from project
    let unit_scale = if let Some(proj_id) = project_id {
        match ifc_lite_core::extract_length_unit_scale(&mut decoder, proj_id) {
            Ok(scale) => scale as f32,
            Err(_) => 1.0,
        }
    } else {
        1.0
    };

    // Apply unit scale to elevations
    for info in spatial_entities.values_mut() {
        if let Some(ref mut elev) = info.elevation {
            *elev *= unit_scale;
        }
    }

    // ============ Second Pass: Process geometry ============
    let router = GeometryRouter::with_units(content, &mut decoder);
    let mut meshes = Vec::new();
    let mut entities = Vec::new();
    let mut scanner = EntityScanner::new(content);

    // Bounds tracking
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];

    // Track which entities have geometry
    let mut entities_with_geometry: HashSet<u64> = HashSet::new();

    // Collect elements with geometry
    let mut element_ids: Vec<(u32, String)> = Vec::new();

    while let Some((id, type_name, _, _)) = scanner.next_entity() {
        if ifc_lite_core::has_geometry_by_name(type_name) {
            let ifc_type = ifc_lite_core::IfcType::from_str(type_name);
            if !matches!(ifc_type, ifc_lite_core::IfcType::Unknown(_)) {
                element_ids.push((id, type_name.to_string()));
            }
        }
    }

    // Process each element
    for (id, type_name) in element_ids {
        let entity = match decoder.decode_by_id(id) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Get entity name
        let name = entity.get_string(2).map(|s| s.to_string());

        // Look up storey information
        let (storey_name, storey_elevation) = if let Some(&storey_id) = element_to_storey.get(&id) {
            if let Some(storey) = spatial_entities.get(&storey_id) {
                (Some(storey.name.clone()), storey.elevation)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Always add to entities for hierarchy (even if geometry fails)
        entities.push(EntityInfo {
            id: id as u64,
            entity_type: type_name.clone(),
            name: name.clone(),
            global_id: None,
            storey: storey_name,
            storey_elevation,
        });

        // Process geometry
        let mesh = match router.process_element(&entity, &mut decoder) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if mesh.is_empty() {
            continue;
        }

        // Track that this entity has geometry
        entities_with_geometry.insert(id as u64);

        // Update bounds
        for chunk in mesh.positions.chunks(3) {
            if chunk.len() == 3 {
                min[0] = min[0].min(chunk[0]);
                min[1] = min[1].min(chunk[1]);
                min[2] = min[2].min(chunk[2]);
                max[0] = max[0].max(chunk[0]);
                max[1] = max[1].max(chunk[1]);
                max[2] = max[2].max(chunk[2]);
            }
        }

        // Get color for entity type
        let color = get_element_color(&type_name);

        // Debug first few meshes
        if meshes.len() < 3 {
            eprintln!(
                "DEBUG FFI Mesh #{}: positions={}, normals={}, indices={}",
                id,
                mesh.positions.len(),
                mesh.normals.len(),
                mesh.indices.len()
            );
        }

        meshes.push(MeshData {
            entity_id: id as u64,
            entity_type: type_name,
            name,
            positions: mesh.positions,
            normals: mesh.normals,
            indices: mesh.indices,
            color: color.to_vec(),
            transform: vec![
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        });
    }

    eprintln!("DEBUG FFI: Total meshes created: {}", meshes.len());

    // Calculate bounds
    let bounds = if min[0] < max[0] {
        Some(SceneBounds {
            min_x: min[0],
            min_y: min[1],
            min_z: min[2],
            max_x: max[0],
            max_y: max[1],
            max_z: max[2],
        })
    } else {
        None
    };

    // ============ Build spatial tree ============
    // Debug output
    eprintln!(
        "DEBUG FFI: First pass scanned {} entities total",
        entity_count
    );
    eprintln!(
        "DEBUG FFI: Total relationship entities found: {}",
        rel_count
    );
    eprintln!(
        "DEBUG FFI: Found {} spatial entities",
        spatial_entities.len()
    );
    eprintln!(
        "DEBUG FFI: Found {} aggregate relationships",
        aggregates.len()
    );
    eprintln!(
        "DEBUG FFI: Found {} containment relationships",
        contained_in.len()
    );

    // If no relationships found, infer hierarchy from entity types
    // Standard hierarchy: Project -> Site -> Building -> Storey -> Space
    if aggregates.is_empty() && !spatial_entities.is_empty() {
        eprintln!("DEBUG FFI: No relationships found, inferring hierarchy from types");

        // Collect entities by type
        let mut projects: Vec<u32> = Vec::new();
        let mut sites: Vec<u32> = Vec::new();
        let mut buildings: Vec<u32> = Vec::new();
        let mut storeys: Vec<u32> = Vec::new();
        let mut spaces: Vec<u32> = Vec::new();

        for (id, info) in &spatial_entities {
            match info.entity_type.to_uppercase().as_str() {
                "IFCPROJECT" => projects.push(*id),
                "IFCSITE" => sites.push(*id),
                "IFCBUILDING" => buildings.push(*id),
                "IFCBUILDINGSTOREY" => storeys.push(*id),
                "IFCSPACE" => spaces.push(*id),
                _ => {}
            }
        }

        // Build inferred hierarchy
        // Project -> Sites
        for &proj_id in &projects {
            if !sites.is_empty() {
                aggregates.entry(proj_id).or_default().extend(sites.clone());
            } else if !buildings.is_empty() {
                // If no sites, project contains buildings directly
                aggregates
                    .entry(proj_id)
                    .or_default()
                    .extend(buildings.clone());
            }
        }

        // Sites -> Buildings
        for &site_id in &sites {
            if !buildings.is_empty() {
                aggregates
                    .entry(site_id)
                    .or_default()
                    .extend(buildings.clone());
            }
        }

        // Buildings -> Storeys
        for &building_id in &buildings {
            if !storeys.is_empty() {
                aggregates
                    .entry(building_id)
                    .or_default()
                    .extend(storeys.clone());
            }
        }

        // Storeys -> Spaces (add spaces as contained elements)
        // Distribute spaces to storeys based on naming or just first storey
        if !spaces.is_empty() && !storeys.is_empty() {
            // For now, put all spaces under the first storey with "00" or "ground" in name
            // or just the first storey if none match
            let ground_storey = storeys
                .iter()
                .find(|&&id| {
                    spatial_entities
                        .get(&id)
                        .map(|info| {
                            let name = info.name.to_lowercase();
                            name.contains("00")
                                || name.contains("ground")
                                || name.contains("erdgeschoss")
                        })
                        .unwrap_or(false)
                })
                .or(storeys.first())
                .copied();

            if let Some(storey_id) = ground_storey {
                contained_in
                    .entry(storey_id)
                    .or_default()
                    .extend(spaces.clone());
            }
        }

        eprintln!(
            "DEBUG FFI: Inferred {} aggregate relationships",
            aggregates.len()
        );
        eprintln!(
            "DEBUG FFI: projects={}, sites={}, buildings={}, storeys={}, spaces={}",
            projects.len(),
            sites.len(),
            buildings.len(),
            storeys.len(),
            spaces.len()
        );
        for (parent, children) in &aggregates {
            if let Some(p_info) = spatial_entities.get(parent) {
                eprintln!(
                    "DEBUG FFI:   {} ({}) -> {} children",
                    p_info.name,
                    p_info.entity_type,
                    children.len()
                );
            }
        }
    }

    let spatial_tree = build_spatial_tree(
        &spatial_entities,
        &aggregates,
        &contained_in,
        &entities,
        &entities_with_geometry,
    );

    eprintln!("DEBUG FFI: spatial_tree = {:?}", spatial_tree.is_some());

    Ok((meshes, entities, spatial_tree, bounds))
}

/// Get node type string from entity type
fn get_node_type(entity_type: &str) -> &'static str {
    match entity_type.to_uppercase().as_str() {
        "IFCPROJECT" => "Project",
        "IFCSITE" => "Site",
        "IFCBUILDING" => "Building",
        "IFCBUILDINGSTOREY" => "Storey",
        "IFCSPACE" => "Space",
        _ => "Element",
    }
}

/// Build spatial tree from collected data
fn build_spatial_tree(
    spatial_entities: &std::collections::HashMap<u32, SpatialInfo>,
    aggregates: &std::collections::HashMap<u32, Vec<u32>>,
    contained_in: &std::collections::HashMap<u32, Vec<u32>>,
    entities: &[EntityInfo],
    entities_with_geometry: &HashSet<u64>,
) -> Option<SpatialNode> {
    // Find root (usually IfcProject)
    let root_id = spatial_entities
        .iter()
        .find(|(_, info)| info.entity_type.to_uppercase() == "IFCPROJECT")
        .map(|(id, _)| *id)?;

    build_node(
        root_id,
        spatial_entities,
        aggregates,
        contained_in,
        entities,
        entities_with_geometry,
    )
}

/// Recursively build a spatial node
fn build_node(
    id: u32,
    spatial_entities: &std::collections::HashMap<u32, SpatialInfo>,
    aggregates: &std::collections::HashMap<u32, Vec<u32>>,
    contained_in: &std::collections::HashMap<u32, Vec<u32>>,
    entities: &[EntityInfo],
    entities_with_geometry: &HashSet<u64>,
) -> Option<SpatialNode> {
    let info = spatial_entities.get(&id)?;
    let node_type = get_node_type(&info.entity_type);

    let mut children: Vec<SpatialNode> = Vec::new();

    // Add aggregated children (Site->Building->Storey hierarchy)
    if let Some(child_ids) = aggregates.get(&id) {
        for &child_id in child_ids {
            if let Some(child_node) = build_node(
                child_id,
                spatial_entities,
                aggregates,
                contained_in,
                entities,
                entities_with_geometry,
            ) {
                children.push(child_node);
            }
        }
    }

    // Add contained elements (elements in this storey/space)
    if let Some(element_ids) = contained_in.get(&id) {
        for &elem_id in element_ids {
            // Find the entity data for this element
            if let Some(elem) = entities.iter().find(|e| e.id == elem_id as u64) {
                let has_geometry = entities_with_geometry.contains(&(elem_id as u64));
                children.push(SpatialNode {
                    id: elem_id as u64,
                    node_type: "Element".to_string(),
                    name: elem.name.clone().unwrap_or_else(|| format!("#{}", elem_id)),
                    entity_type: elem.entity_type.clone(),
                    elevation: None,
                    has_geometry,
                    children: Vec::new(),
                });
            }
        }
    }

    // Sort children: spatial structures first (by elevation desc), then elements by type/name
    children.sort_by(|a, b| {
        // Spatial structures come first
        let a_is_spatial = a.node_type != "Element";
        let b_is_spatial = b.node_type != "Element";
        if a_is_spatial != b_is_spatial {
            return b_is_spatial.cmp(&a_is_spatial);
        }
        // For storeys, sort by elevation (descending)
        if a.node_type == "Storey" && b.node_type == "Storey" {
            return b
                .elevation
                .partial_cmp(&a.elevation)
                .unwrap_or(std::cmp::Ordering::Equal);
        }
        // Otherwise sort by type then name
        match a.entity_type.cmp(&b.entity_type) {
            std::cmp::Ordering::Equal => a.name.cmp(&b.name),
            other => other,
        }
    });

    Some(SpatialNode {
        id: id as u64,
        node_type: node_type.to_string(),
        name: info.name.clone(),
        entity_type: info.entity_type.clone(),
        elevation: info.elevation,
        has_geometry: false, // Spatial structures don't have geometry
        children,
    })
}

/// Get default color for entity type
fn get_element_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();

    if upper.contains("WALL") {
        [0.95, 0.92, 0.85, 1.0] // Warm off-white
    } else if upper.contains("SLAB") || upper.contains("FLOOR") {
        [0.75, 0.75, 0.78, 1.0] // Cool gray
    } else if upper.contains("ROOF") {
        [0.7, 0.45, 0.35, 1.0] // Terracotta
    } else if upper.contains("BEAM") || upper.contains("COLUMN") {
        [0.55, 0.55, 0.6, 1.0] // Steel blue-gray
    } else if upper.contains("DOOR") {
        [0.6, 0.45, 0.3, 1.0] // Warm wood
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.6, 0.8, 0.95, 0.4] // Sky blue, transparent
    } else if upper.contains("STAIR") || upper.contains("RAILING") {
        [0.65, 0.6, 0.55, 1.0] // Warm gray
    } else if upper.contains("COVERING") {
        [0.9, 0.88, 0.82, 1.0] // Light cream
    } else if upper.contains("FURNITURE") || upper.contains("FURNISHING") {
        [0.7, 0.55, 0.4, 1.0] // Light wood
    } else if upper.contains("PIPE") || upper.contains("DUCT") || upper.contains("CABLE") {
        [0.4, 0.6, 0.4, 1.0] // Industrial green
    } else {
        [0.8, 0.78, 0.75, 1.0] // Default warm gray
    }
}

/// Extract properties for a specific entity
fn extract_properties(content: &str, entity_id: u32) -> Vec<PropertySet> {
    use ifc_lite_core::{build_entity_index, EntityDecoder, EntityScanner};

    let index = build_entity_index(content);
    let mut decoder = EntityDecoder::with_index(content, index);

    // Step 1: Find all IFCRELDEFINESBYPROPERTIES that reference this entity
    let mut property_set_ids: Vec<u32> = Vec::new();

    let mut scanner = EntityScanner::new(content);
    while let Some((id, type_name, _, _)) = scanner.next_entity() {
        if type_name.to_uppercase() == "IFCRELDEFINESBYPROPERTIES" {
            if let Ok(entity) = decoder.decode_by_id(id) {
                // RelatedObjects is at index 4 (list of entity refs)
                if let Some(related) = get_ref_list(&entity, 4) {
                    if related.contains(&entity_id) {
                        // RelatingPropertyDefinition is at index 5
                        if let Some(pset_id) = entity.get_ref(5) {
                            property_set_ids.push(pset_id);
                        }
                    }
                }
            }
        }
    }

    // Step 2: For each property set ID, extract the property set and its properties
    let mut result: Vec<PropertySet> = Vec::new();

    for pset_id in property_set_ids {
        if let Ok(pset_entity) = decoder.decode_by_id(pset_id) {
            let pset_type = pset_entity.ifc_type.to_string().to_uppercase();

            if pset_type == "IFCPROPERTYSET" {
                // Name is at index 2
                let pset_name = pset_entity
                    .get_string(2)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("PropertySet #{}", pset_id));

                // HasProperties is at index 4 (list of property refs)
                let mut properties: Vec<PropertyValue> = Vec::new();

                if let Some(prop_ids) = get_ref_list(&pset_entity, 4) {
                    for prop_id in prop_ids {
                        if let Ok(prop_entity) = decoder.decode_by_id(prop_id) {
                            let prop_type = prop_entity.ifc_type.to_string().to_uppercase();

                            if prop_type == "IFCPROPERTYSINGLEVALUE" {
                                // Name at index 0, NominalValue at index 2
                                let prop_name = prop_entity
                                    .get_string(0)
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| format!("Property #{}", prop_id));

                                // Extract value - could be various IFC types
                                let prop_value = extract_property_value(&prop_entity, 2);

                                // Unit at index 3 (optional)
                                let unit = prop_entity.get_string(3).map(|s| s.to_string());

                                properties.push(PropertyValue {
                                    name: prop_name,
                                    value: prop_value,
                                    unit,
                                });
                            }
                        }
                    }
                }

                if !properties.is_empty() {
                    result.push(PropertySet {
                        name: pset_name,
                        properties,
                    });
                }
            } else if pset_type == "IFCELEMENTQUANTITY" {
                // IfcElementQuantity for quantities
                let pset_name = pset_entity
                    .get_string(2)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("Quantities #{}", pset_id));

                let mut properties: Vec<PropertyValue> = Vec::new();

                // Quantities at index 5
                if let Some(qty_ids) = get_ref_list(&pset_entity, 5) {
                    for qty_id in qty_ids {
                        if let Ok(qty_entity) = decoder.decode_by_id(qty_id) {
                            // Name at index 0
                            let qty_name = qty_entity
                                .get_string(0)
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| format!("Quantity #{}", qty_id));

                            // Value depends on quantity type
                            let qty_value = extract_quantity_value(&qty_entity);

                            properties.push(PropertyValue {
                                name: qty_name,
                                value: qty_value,
                                unit: None,
                            });
                        }
                    }
                }

                if !properties.is_empty() {
                    result.push(PropertySet {
                        name: pset_name,
                        properties,
                    });
                }
            }
        }
    }

    result
}

/// Extract value from a property entity at given index
fn extract_property_value(entity: &ifc_lite_core::DecodedEntity, index: usize) -> String {
    use ifc_lite_core::AttributeValue;

    if let Some(attr) = entity.get(index) {
        match attr {
            AttributeValue::String(s) => return s.clone(),
            AttributeValue::Float(f) => return format!("{:.4}", f),
            AttributeValue::Integer(i) => return i.to_string(),
            AttributeValue::Enum(e) => return e.clone(),
            AttributeValue::List(list) => {
                // For wrapped types like IFCLABEL('value')
                if let Some(AttributeValue::String(s)) = list.first() {
                    return s.clone();
                }
                if let Some(AttributeValue::Float(f)) = list.first() {
                    return format!("{:.4}", f);
                }
                if let Some(AttributeValue::Integer(i)) = list.first() {
                    return i.to_string();
                }
            }
            AttributeValue::Null | AttributeValue::Derived => return "—".to_string(),
            AttributeValue::EntityRef(_) => return "—".to_string(),
        }
    }

    "—".to_string()
}

/// Extract value from a quantity entity
fn extract_quantity_value(entity: &ifc_lite_core::DecodedEntity) -> String {
    use ifc_lite_core::AttributeValue;

    let qty_type = entity.ifc_type.to_string().to_uppercase();

    // Different quantity types have value at different indices
    let value_index = match qty_type.as_str() {
        "IFCQUANTITYLENGTH" => 3,
        "IFCQUANTITYAREA" => 3,
        "IFCQUANTITYVOLUME" => 3,
        "IFCQUANTITYCOUNT" => 3,
        "IFCQUANTITYWEIGHT" => 3,
        "IFCQUANTITYTIME" => 3,
        _ => 3,
    };

    if let Some(attr) = entity.get(value_index) {
        match attr {
            AttributeValue::Float(f) => return format!("{:.4}", *f),
            AttributeValue::Integer(i) => return i.to_string(),
            _ => {}
        }
    }

    "—".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_scene() {
        let scene = IfcScene::new();
        assert!(!scene.is_loaded());
    }

    #[test]
    fn test_selection() {
        let scene = IfcScene::new();
        scene.select(1);
        assert_eq!(scene.get_selection().selected_ids, vec![1]);

        scene.add_to_selection(2);
        let sel = scene.get_selection();
        assert!(sel.selected_ids.contains(&1));
        assert!(sel.selected_ids.contains(&2));

        scene.clear_selection();
        assert!(scene.get_selection().selected_ids.is_empty());
    }

    #[test]
    fn test_spatial_tree() {
        let content = std::fs::read_to_string("../../tests/models/test.ifc")
            .expect("Failed to read test.ifc");

        let (meshes, entities, spatial_tree, bounds) =
            process_ifc_content(&content).expect("Failed to process IFC");

        println!("Meshes: {}", meshes.len());
        println!("Entities: {}", entities.len());
        println!("Bounds: {:?}", bounds);
        println!("Spatial tree: {:?}", spatial_tree.is_some());

        if let Some(ref tree) = spatial_tree {
            println!("Root: {} ({})", tree.name, tree.node_type);
            println!("Children: {}", tree.children.len());
            for child in &tree.children {
                println!(
                    "  - {} ({}) with {} children",
                    child.name,
                    child.node_type,
                    child.children.len()
                );
            }
        }

        assert!(spatial_tree.is_some(), "Spatial tree should be built");
    }

    #[test]
    fn test_spatial_tree_duplex() {
        let content = std::fs::read_to_string("../../tests/models/ara3d/duplex.ifc")
            .expect("Failed to read duplex.ifc");

        println!("File size: {} bytes", content.len());

        let (meshes, entities, spatial_tree, bounds) =
            process_ifc_content(&content).expect("Failed to process IFC");

        println!("Meshes: {}", meshes.len());
        println!("Entities: {}", entities.len());
        println!("Bounds: {:?}", bounds);
        println!("Spatial tree: {:?}", spatial_tree.is_some());

        if let Some(ref tree) = spatial_tree {
            println!("Root: {} ({})", tree.name, tree.node_type);
            println!("Children: {}", tree.children.len());
            for child in &tree.children {
                println!(
                    "  - {} ({}) with {} children",
                    child.name,
                    child.node_type,
                    child.children.len()
                );
                for grandchild in &child.children {
                    println!(
                        "    - {} ({}) with {} children",
                        grandchild.name,
                        grandchild.node_type,
                        grandchild.children.len()
                    );
                }
            }
        }

        assert!(
            spatial_tree.is_some(),
            "Spatial tree should be built for duplex.ifc"
        );
    }
}
