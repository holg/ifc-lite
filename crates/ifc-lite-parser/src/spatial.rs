// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Spatial structure builder and query implementation

use ifc_lite_model::{
    AttributeValue, DecodedEntity, EntityId, EntityResolver, IfcType, SpatialNode, SpatialNodeType,
    SpatialQuery, StoreyInfo,
};
use rustc_hash::FxHashMap;

/// Spatial query implementation
pub struct SpatialQueryImpl {
    /// Root of the spatial tree
    tree: Option<SpatialNode>,
    /// Storey information cache
    storeys: Vec<StoreyInfo>,
    /// Element to storey mapping
    element_storey_map: FxHashMap<u32, EntityId>,
    /// Entity type index for search
    type_index: FxHashMap<IfcType, Vec<EntityId>>,
    /// Name index for search (lowercase name -> entity IDs)
    name_index: FxHashMap<String, Vec<EntityId>>,
}

impl SpatialQueryImpl {
    /// Build spatial structure from resolver
    pub fn build(resolver: &dyn EntityResolver) -> Self {
        let mut builder = SpatialBuilder::new(resolver);
        builder.build();

        Self {
            tree: builder.tree,
            storeys: builder.storeys,
            element_storey_map: builder.element_storey_map,
            type_index: builder.type_index,
            name_index: builder.name_index,
        }
    }

    /// Create an empty spatial query
    pub fn empty() -> Self {
        Self {
            tree: None,
            storeys: Vec::new(),
            element_storey_map: FxHashMap::default(),
            type_index: FxHashMap::default(),
            name_index: FxHashMap::default(),
        }
    }
}

impl SpatialQuery for SpatialQueryImpl {
    fn spatial_tree(&self) -> Option<&SpatialNode> {
        self.tree.as_ref()
    }

    fn storeys(&self) -> Vec<StoreyInfo> {
        self.storeys.clone()
    }

    fn elements_in_storey(&self, storey_id: EntityId) -> Vec<EntityId> {
        // Find the storey in the tree and return its elements
        if let Some(tree) = &self.tree {
            if let Some(storey) = tree.find(storey_id) {
                return storey.element_ids();
            }
        }
        Vec::new()
    }

    fn containing_storey(&self, element_id: EntityId) -> Option<EntityId> {
        self.element_storey_map.get(&element_id.0).copied()
    }

    fn search(&self, query: &str) -> Vec<EntityId> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // Search by name
        for (name, ids) in &self.name_index {
            if name.contains(&query_lower) {
                results.extend(ids.iter().copied());
            }
        }

        // Search by type name
        let query_upper = query.to_uppercase();
        for (ifc_type, ids) in &self.type_index {
            let type_name = format!("{:?}", ifc_type);
            if type_name.to_uppercase().contains(&query_upper) {
                results.extend(ids.iter().copied());
            }
        }

        // Deduplicate
        results.sort_by_key(|id| id.0);
        results.dedup();
        results
    }

    fn elements_by_type(&self, ifc_type: &IfcType) -> Vec<EntityId> {
        self.type_index.get(ifc_type).cloned().unwrap_or_default()
    }
}

/// Helper struct for building spatial structure
struct SpatialBuilder<'a> {
    resolver: &'a dyn EntityResolver,
    tree: Option<SpatialNode>,
    storeys: Vec<StoreyInfo>,
    element_storey_map: FxHashMap<u32, EntityId>,
    type_index: FxHashMap<IfcType, Vec<EntityId>>,
    name_index: FxHashMap<String, Vec<EntityId>>,
    /// Entities with geometry representation
    entities_with_geometry: FxHashMap<u32, bool>,
}

impl<'a> SpatialBuilder<'a> {
    fn new(resolver: &'a dyn EntityResolver) -> Self {
        Self {
            resolver,
            tree: None,
            storeys: Vec::new(),
            element_storey_map: FxHashMap::default(),
            type_index: FxHashMap::default(),
            name_index: FxHashMap::default(),
            entities_with_geometry: FxHashMap::default(),
        }
    }

    fn build(&mut self) {
        // Build geometry presence cache first
        self.build_geometry_cache();

        // Build type and name indices
        self.build_indices();

        // Find project (root)
        let projects = self.resolver.entities_by_type(&IfcType::IfcProject);
        if projects.is_empty() {
            return;
        }

        let project = &projects[0];
        let mut root = self.create_node(project);

        // Build hierarchy from project down
        self.add_spatial_children(&mut root, project.id);

        // Extract storey info
        self.extract_storeys(&root);

        self.tree = Some(root);
    }

    fn build_geometry_cache(&mut self) {
        // Find all IFCPRODUCTDEFINITIONSHAPE and mark their products as having geometry
        for _shape in self
            .resolver
            .entities_by_type(&IfcType::IfcProductDefinitionShape)
        {
            // The shape is referenced by products through their Representation attribute
            // We'll mark by checking what refers to this shape
        }

        // Simpler approach: check products directly for Representation attribute
        let product_types = [
            IfcType::IfcWall,
            IfcType::IfcWallStandardCase,
            IfcType::IfcSlab,
            IfcType::IfcBeam,
            IfcType::IfcColumn,
            IfcType::IfcDoor,
            IfcType::IfcWindow,
            IfcType::IfcStair,
            IfcType::IfcStairFlight,
            IfcType::IfcRoof,
            IfcType::IfcCovering,
            IfcType::IfcRailing,
            IfcType::IfcPlate,
            IfcType::IfcMember,
            IfcType::IfcCurtainWall,
            IfcType::IfcFooting,
            IfcType::IfcPile,
            IfcType::IfcBuildingElementProxy,
            IfcType::IfcOpeningElement,
            IfcType::IfcFurnishingElement,
            IfcType::IfcFlowTerminal,
            IfcType::IfcFlowSegment,
            IfcType::IfcFlowFitting,
        ];

        for ifc_type in product_types {
            for entity in self.resolver.entities_by_type(&ifc_type) {
                // Representation is typically at index 6 for most products
                let has_rep = entity.get_ref(6).is_some();
                self.entities_with_geometry.insert(entity.id.0, has_rep);
            }
        }
    }

    fn build_indices(&mut self) {
        // Build indices for all entities
        for id in self.resolver.all_ids() {
            if let Some(entity) = self.resolver.get(id) {
                // Type index
                self.type_index
                    .entry(entity.ifc_type.clone())
                    .or_default()
                    .push(id);

                // Name index (Name is typically at index 2 for most entities)
                if let Some(name) = entity.get_string(2) {
                    let name_lower = name.to_lowercase();
                    self.name_index.entry(name_lower).or_default().push(id);
                }
            }
        }
    }

    fn create_node(&self, entity: &DecodedEntity) -> SpatialNode {
        let node_type = SpatialNodeType::from_ifc_type(&entity.ifc_type);
        let name = entity.get_string(2).unwrap_or("").to_string();
        let entity_type = format!("{:?}", entity.ifc_type);
        let has_geometry = self
            .entities_with_geometry
            .get(&entity.id.0)
            .copied()
            .unwrap_or(false);

        let mut node = SpatialNode::new(entity.id, node_type, name, entity_type)
            .with_geometry(has_geometry);

        // Extract elevation for storeys
        if entity.ifc_type == IfcType::IfcBuildingStorey {
            if let Some(elevation) = entity.get_float(9) {
                node = node.with_elevation(elevation as f32);
            }
        }

        node
    }

    fn add_spatial_children(&mut self, parent: &mut SpatialNode, parent_id: EntityId) {
        // Find IFCRELAGGREGATES where parent is RelatingObject
        for rel in self
            .resolver
            .entities_by_type(&IfcType::IfcRelAggregates)
        {
            // RelatingObject at index 4
            if rel.get_ref(4) != Some(parent_id) {
                continue;
            }

            // RelatedObjects at index 5
            let children = match rel.get(5) {
                Some(AttributeValue::List(list)) => list,
                _ => continue,
            };

            for child_ref in children {
                if let AttributeValue::EntityRef(child_id) = child_ref {
                    if let Some(child_entity) = self.resolver.get(*child_id) {
                        let mut child_node = self.create_node(&child_entity);
                        self.add_spatial_children(&mut child_node, *child_id);
                        parent.add_child(child_node);
                    }
                }
            }
        }

        // Find IFCRELCONTAINEDINSPATIALSTRUCTURE where parent is RelatingStructure
        for rel in self
            .resolver
            .entities_by_type(&IfcType::IfcRelContainedInSpatialStructure)
        {
            // RelatingStructure at index 5
            if rel.get_ref(5) != Some(parent_id) {
                continue;
            }

            // RelatedElements at index 4
            let elements = match rel.get(4) {
                Some(AttributeValue::List(list)) => list,
                _ => continue,
            };

            for elem_ref in elements {
                if let AttributeValue::EntityRef(elem_id) = elem_ref {
                    if let Some(elem_entity) = self.resolver.get(*elem_id) {
                        let child_node = self.create_node(&elem_entity);

                        // Track element to storey mapping
                        if parent.node_type == SpatialNodeType::Storey {
                            self.element_storey_map.insert(elem_id.0, parent_id);
                        }

                        parent.add_child(child_node);
                    }
                }
            }
        }
    }

    fn extract_storeys(&mut self, tree: &SpatialNode) {
        // Collect all storeys from the tree
        for node in tree.iter() {
            if node.node_type == SpatialNodeType::Storey {
                let element_count = node.element_count();
                self.storeys.push(StoreyInfo::new(
                    node.id,
                    &node.name,
                    node.elevation.unwrap_or(0.0),
                    element_count,
                ));
            }
        }

        // Sort by elevation
        self.storeys
            .sort_by(|a, b| a.elevation.partial_cmp(&b.elevation).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require a mock resolver
    #[test]
    fn test_empty_spatial() {
        let spatial = SpatialQueryImpl::empty();
        assert!(spatial.spatial_tree().is_none());
        assert!(spatial.storeys().is_empty());
    }
}
