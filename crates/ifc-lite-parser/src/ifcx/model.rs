// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! IFC5 (IFCX) model implementation
//!
//! Implements IfcModel trait for IFCX JSON format.

use super::composition::compose_nodes;
use super::types::{attr, ComposedNode, IfcClass, IfcxFile};
use crate::Result;
use ifc_lite_model::{
    AttributeValue, DecodedEntity, EntityId, EntityResolver, IfcModel, IfcType, ModelMetadata,
    Property, PropertyReader, PropertySet, Quantity, SpatialNode, SpatialNodeType, SpatialQuery,
    StoreyInfo,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// IFC5 (IFCX) model
pub struct IfcxModel {
    /// Composed nodes by path
    nodes: FxHashMap<String, ComposedNode>,
    /// Path to synthetic ID mapping
    path_to_id: FxHashMap<String, EntityId>,
    /// ID to path mapping
    id_to_path: FxHashMap<EntityId, String>,
    /// Type index for fast type lookups
    type_index: FxHashMap<IfcType, Vec<EntityId>>,
    /// Decoded entities cache
    entities: FxHashMap<EntityId, Arc<DecodedEntity>>,
    /// Spatial tree
    spatial_tree: Option<SpatialNode>,
    /// Model metadata
    metadata: ModelMetadata,
    /// Unit scale (default 1.0 for meters)
    unit_scale: f64,
}

impl IfcxModel {
    /// Parse IFCX JSON content
    pub fn parse(content: &str) -> Result<Self> {
        // Parse JSON
        let file: IfcxFile = serde_json::from_str(content)
            .map_err(|e| ifc_lite_model::ParseError::InvalidFormat(e.to_string()))?;

        // Compose nodes (flatten ECS)
        let nodes = compose_nodes(&file.data);

        // Build path<->ID mappings
        let mut path_to_id = FxHashMap::default();
        let mut id_to_path = FxHashMap::default();
        let mut next_id = 1u32;

        for path in nodes.keys() {
            let id = EntityId(next_id);
            path_to_id.insert(path.clone(), id);
            id_to_path.insert(id, path.clone());
            next_id += 1;
        }

        // Build type index and entities
        let mut type_index: FxHashMap<IfcType, Vec<EntityId>> = FxHashMap::default();
        let mut entities = FxHashMap::default();

        for (path, node) in &nodes {
            let id = path_to_id[path];
            let ifc_type = extract_ifc_type(node);

            // Build decoded entity
            let entity = Arc::new(DecodedEntity {
                id,
                ifc_type: ifc_type.clone(),
                attributes: build_attributes(node, &path_to_id),
            });

            entities.insert(id, entity);
            type_index.entry(ifc_type).or_default().push(id);
        }

        // Build spatial tree
        let spatial_tree = build_spatial_tree(&nodes, &path_to_id);

        // Extract metadata
        let metadata = ModelMetadata {
            schema_version: format!("IFC5 ({})", file.header.ifcx_version),
            originating_system: Some(file.header.author.clone()),
            file_name: Some(file.header.id.clone()),
            timestamp: Some(file.header.timestamp.clone()),
            ..Default::default()
        };

        Ok(Self {
            nodes,
            path_to_id,
            id_to_path,
            type_index,
            entities,
            spatial_tree,
            metadata,
            unit_scale: 1.0, // IFCX uses meters by default
        })
    }

    /// Get node by entity ID
    pub fn node(&self, id: EntityId) -> Option<&ComposedNode> {
        let path = self.id_to_path.get(&id)?;
        self.nodes.get(path)
    }

    /// Get path for entity ID
    pub fn path(&self, id: EntityId) -> Option<&str> {
        self.id_to_path.get(&id).map(|s| s.as_str())
    }

    /// Get entity ID for path
    pub fn id_for_path(&self, path: &str) -> Option<EntityId> {
        self.path_to_id.get(path).copied()
    }
}

/// Extract IFC type from composed node
fn extract_ifc_type(node: &ComposedNode) -> IfcType {
    if let Some(class_val) = node.attributes.get(attr::CLASS) {
        if let Some(class) = IfcClass::from_value(class_val) {
            return IfcType::parse(&class.code);
        }
    }
    IfcType::Unknown(String::new())
}

/// Build attribute values from composed node
fn build_attributes(
    node: &ComposedNode,
    path_to_id: &FxHashMap<String, EntityId>,
) -> Vec<AttributeValue> {
    // For IFCX, we store key attributes in a standardized order:
    // [0] = GlobalId (path/UUID)
    // [1] = OwnerHistory (null for IFCX)
    // [2] = Name
    // [3] = Description
    // [4] = ObjectType
    // [5] = Children refs

    let mut attrs = vec![AttributeValue::Null; 10];

    // GlobalId = path
    attrs[0] = AttributeValue::String(node.path.clone());

    // OwnerHistory = null
    attrs[1] = AttributeValue::Null;

    // Name - check various property patterns
    if let Some(name) = node
        .attributes
        .get("bsi::ifc::prop::Name")
        .or_else(|| node.attributes.get("bsi::ifc::prop::TypeName"))
    {
        if let Some(s) = name.as_str() {
            attrs[2] = AttributeValue::String(s.to_string());
        }
    }

    // Description
    if let Some(desc) = node.attributes.get("bsi::ifc::prop::Description") {
        if let Some(s) = desc.as_str() {
            attrs[3] = AttributeValue::String(s.to_string());
        }
    }

    // Children as entity refs
    let child_refs: Vec<AttributeValue> = node
        .children
        .iter()
        .filter_map(|child_path| {
            path_to_id
                .get(child_path)
                .map(|id| AttributeValue::EntityRef(*id))
        })
        .collect();

    if !child_refs.is_empty() {
        attrs[5] = AttributeValue::List(child_refs);
    }

    attrs
}

/// Build spatial tree from composed nodes
fn build_spatial_tree(
    nodes: &FxHashMap<String, ComposedNode>,
    path_to_id: &FxHashMap<String, EntityId>,
) -> Option<SpatialNode> {
    // Find root (usually IfcProject)
    let mut root_path: Option<&str> = None;

    for (path, node) in nodes {
        let ifc_type = extract_ifc_type(node);
        if matches!(ifc_type, IfcType::IfcProject) {
            root_path = Some(path);
            break;
        }
    }

    // If no project, find node with no parent
    if root_path.is_none() {
        for (path, node) in nodes {
            if node.parent.is_none() && !node.children.is_empty() {
                root_path = Some(path);
                break;
            }
        }
    }

    let root_path = root_path?;

    // Build tree recursively
    fn build_node(
        path: &str,
        nodes: &FxHashMap<String, ComposedNode>,
        path_to_id: &FxHashMap<String, EntityId>,
    ) -> Option<SpatialNode> {
        let node = nodes.get(path)?;
        let id = *path_to_id.get(path)?;
        let ifc_type = extract_ifc_type(node);

        // Get name
        let name = node
            .attributes
            .get("bsi::ifc::prop::Name")
            .or_else(|| node.attributes.get("bsi::ifc::prop::TypeName"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| path.to_string());

        let node_type = SpatialNodeType::from_ifc_type(&ifc_type);
        let entity_type = ifc_type.name().to_string();

        // Check if has geometry
        let has_geometry = node.attributes.contains_key(attr::MESH);

        // Build children recursively
        let children: Vec<SpatialNode> = node
            .children
            .iter()
            .filter_map(|child_path| build_node(child_path, nodes, path_to_id))
            .collect();

        let mut spatial_node = SpatialNode::new(id, node_type, name, entity_type);
        spatial_node.children = children;
        spatial_node.has_geometry = has_geometry;

        Some(spatial_node)
    }

    build_node(root_path, nodes, path_to_id)
}

// Implement IfcModel trait
impl IfcModel for IfcxModel {
    fn resolver(&self) -> &dyn EntityResolver {
        self
    }

    fn properties(&self) -> &dyn PropertyReader {
        self
    }

    fn spatial(&self) -> &dyn SpatialQuery {
        self
    }

    fn unit_scale(&self) -> f64 {
        self.unit_scale
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }
}

// Implement EntityResolver
impl EntityResolver for IfcxModel {
    fn get(&self, id: EntityId) -> Option<Arc<DecodedEntity>> {
        self.entities.get(&id).cloned()
    }

    fn entities_by_type(&self, ifc_type: &IfcType) -> Vec<Arc<DecodedEntity>> {
        self.type_index
            .get(ifc_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.entities.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn find_by_type_name(&self, type_name: &str) -> Vec<Arc<DecodedEntity>> {
        let target = IfcType::parse(type_name);
        self.entities_by_type(&target)
    }

    fn count_by_type(&self, ifc_type: &IfcType) -> usize {
        self.type_index.get(ifc_type).map(|v| v.len()).unwrap_or(0)
    }

    fn all_ids(&self) -> Vec<EntityId> {
        self.entities.keys().copied().collect()
    }

    fn raw_bytes(&self, _id: EntityId) -> Option<&[u8]> {
        // Not applicable for JSON format
        None
    }
}

// Implement PropertyReader
impl PropertyReader for IfcxModel {
    fn property_sets(&self, id: EntityId) -> Vec<PropertySet> {
        let Some(node) = self.node(id) else {
            return Vec::new();
        };

        // Group attributes by namespace prefix as "property sets"
        let mut psets: FxHashMap<String, Vec<Property>> = FxHashMap::default();

        for (key, value) in &node.attributes {
            // Skip non-property attributes
            if key.starts_with("usd::") || key == attr::CLASS || key == attr::MATERIAL {
                continue;
            }

            // Extract namespace and property name
            let (namespace, prop_name) = if let Some(pos) = key.rfind("::") {
                (key[..pos].to_string(), key[pos + 2..].to_string())
            } else {
                ("Properties".to_string(), key.clone())
            };

            // Convert JSON value to string
            let prop_value = json_to_string(value);

            psets
                .entry(namespace)
                .or_default()
                .push(Property::new(prop_name, prop_value));
        }

        psets
            .into_iter()
            .map(|(name, properties)| PropertySet { name, properties })
            .collect()
    }

    fn quantities(&self, _id: EntityId) -> Vec<Quantity> {
        // Quantities in IFCX are just namespaced properties
        // Could filter for "bsi::ifc::qto::" prefix
        Vec::new()
    }

    fn global_id(&self, id: EntityId) -> Option<String> {
        // GlobalId is the path (UUID)
        self.path(id).map(String::from)
    }

    fn name(&self, id: EntityId) -> Option<String> {
        let node = self.node(id)?;
        node.attributes
            .get("bsi::ifc::prop::Name")
            .or_else(|| node.attributes.get("bsi::ifc::prop::TypeName"))
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn description(&self, id: EntityId) -> Option<String> {
        let node = self.node(id)?;
        node.attributes
            .get("bsi::ifc::prop::Description")
            .and_then(|v| v.as_str())
            .map(String::from)
    }
}

/// Convert JSON value to string for property display
fn json_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(json_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            // For objects, try to extract meaningful value
            if let Some(code) = obj.get("code").and_then(|v| v.as_str()) {
                code.to_string()
            } else {
                value.to_string()
            }
        }
        serde_json::Value::Null => "null".to_string(),
    }
}

// Implement SpatialQuery
impl SpatialQuery for IfcxModel {
    fn spatial_tree(&self) -> Option<&SpatialNode> {
        self.spatial_tree.as_ref()
    }

    fn storeys(&self) -> Vec<StoreyInfo> {
        let Some(tree) = &self.spatial_tree else {
            return Vec::new();
        };

        let mut storeys = Vec::new();

        fn find_storeys(node: &SpatialNode, storeys: &mut Vec<StoreyInfo>) {
            if node.node_type == SpatialNodeType::Storey {
                storeys.push(StoreyInfo {
                    id: node.id,
                    name: node.name.clone(),
                    elevation: node.elevation.unwrap_or(0.0),
                    element_count: node.element_count(),
                });
            }
            for child in &node.children {
                find_storeys(child, storeys);
            }
        }

        find_storeys(tree, &mut storeys);
        storeys.sort_by(|a, b| a.elevation.partial_cmp(&b.elevation).unwrap());
        storeys
    }

    fn elements_in_storey(&self, storey_id: EntityId) -> Vec<EntityId> {
        let Some(tree) = &self.spatial_tree else {
            return Vec::new();
        };

        if let Some(storey_node) = tree.find(storey_id) {
            storey_node.element_ids()
        } else {
            Vec::new()
        }
    }

    fn containing_storey(&self, element_id: EntityId) -> Option<EntityId> {
        // Walk up the parent chain from the element
        let path = self.id_to_path.get(&element_id)?;
        let mut current_path = self.nodes.get(path)?.parent.clone();

        while let Some(p) = current_path {
            let node = self.nodes.get(&p)?;
            let ifc_type = extract_ifc_type(node);
            if matches!(ifc_type, IfcType::IfcBuildingStorey) {
                return self.path_to_id.get(&p).copied();
            }
            current_path = node.parent.clone();
        }

        None
    }

    fn search(&self, query: &str) -> Vec<EntityId> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (path, node) in &self.nodes {
            // Search in name
            if let Some(name) = node
                .attributes
                .get("bsi::ifc::prop::Name")
                .or_else(|| node.attributes.get("bsi::ifc::prop::TypeName"))
                .and_then(|v| v.as_str())
            {
                if name.to_lowercase().contains(&query_lower) {
                    if let Some(id) = self.path_to_id.get(path) {
                        results.push(*id);
                        continue;
                    }
                }
            }

            // Search in type
            let ifc_type = extract_ifc_type(node);
            if ifc_type.name().to_lowercase().contains(&query_lower) {
                if let Some(id) = self.path_to_id.get(path) {
                    results.push(*id);
                }
            }
        }

        results
    }

    fn elements_by_type(&self, ifc_type: &IfcType) -> Vec<EntityId> {
        self.type_index.get(ifc_type).cloned().unwrap_or_default()
    }
}
