// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Spatial structure and hierarchy traversal

use crate::{EntityId, IfcType};
use serde::{Deserialize, Serialize};

/// Type of spatial structure node
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpatialNodeType {
    /// IfcProject - root of the hierarchy
    Project,
    /// IfcSite - geographic site
    Site,
    /// IfcBuilding - a building structure
    Building,
    /// IfcBuildingStorey - a floor/level
    Storey,
    /// IfcSpace - a room or area
    Space,
    /// Building element (wall, door, etc.)
    Element,
    /// IFC4x3 Facility (road, bridge, etc.)
    Facility,
    /// IFC4x3 Facility part
    FacilityPart,
}

impl SpatialNodeType {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            SpatialNodeType::Project => "Project",
            SpatialNodeType::Site => "Site",
            SpatialNodeType::Building => "Building",
            SpatialNodeType::Storey => "Storey",
            SpatialNodeType::Space => "Space",
            SpatialNodeType::Element => "Element",
            SpatialNodeType::Facility => "Facility",
            SpatialNodeType::FacilityPart => "Facility Part",
        }
    }

    /// Get icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            SpatialNodeType::Project => "📋",
            SpatialNodeType::Site => "🌍",
            SpatialNodeType::Building => "🏢",
            SpatialNodeType::Storey => "📐",
            SpatialNodeType::Space => "🚪",
            SpatialNodeType::Element => "🧱",
            SpatialNodeType::Facility => "🛣️",
            SpatialNodeType::FacilityPart => "🔧",
        }
    }

    /// Determine node type from IFC type
    pub fn from_ifc_type(ifc_type: &IfcType) -> Self {
        match ifc_type {
            IfcType::IfcProject => SpatialNodeType::Project,
            IfcType::IfcSite => SpatialNodeType::Site,
            IfcType::IfcBuilding => SpatialNodeType::Building,
            IfcType::IfcBuildingStorey => SpatialNodeType::Storey,
            IfcType::IfcSpace => SpatialNodeType::Space,
            IfcType::IfcFacility | IfcType::IfcRoad | IfcType::IfcBridge | IfcType::IfcRailway => {
                SpatialNodeType::Facility
            }
            IfcType::IfcFacilityPart
            | IfcType::IfcRoadPart
            | IfcType::IfcBridgePart
            | IfcType::IfcRailwayPart => SpatialNodeType::FacilityPart,
            _ => SpatialNodeType::Element,
        }
    }
}

/// Node in the spatial hierarchy tree
///
/// Represents an entry in the IFC spatial structure hierarchy.
/// The tree typically follows: Project → Site → Building → Storey → Elements
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpatialNode {
    /// Entity ID
    pub id: EntityId,
    /// Type of spatial node
    pub node_type: SpatialNodeType,
    /// Display name
    pub name: String,
    /// IFC entity type name (e.g., "IfcWall")
    pub entity_type: String,
    /// Elevation (for storeys)
    pub elevation: Option<f32>,
    /// Child nodes
    pub children: Vec<SpatialNode>,
    /// Whether this entity has geometry
    pub has_geometry: bool,
}

impl SpatialNode {
    /// Create a new spatial node
    pub fn new(
        id: EntityId,
        node_type: SpatialNodeType,
        name: impl Into<String>,
        entity_type: impl Into<String>,
    ) -> Self {
        Self {
            id,
            node_type,
            name: name.into(),
            entity_type: entity_type.into(),
            elevation: None,
            children: Vec::new(),
            has_geometry: false,
        }
    }

    /// Set elevation
    pub fn with_elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// Set has_geometry flag
    pub fn with_geometry(mut self, has_geometry: bool) -> Self {
        self.has_geometry = has_geometry;
        self
    }

    /// Add a child node
    pub fn add_child(&mut self, child: SpatialNode) {
        self.children.push(child);
    }

    /// Get total element count (recursive)
    pub fn element_count(&self) -> usize {
        let own = if self.node_type == SpatialNodeType::Element {
            1
        } else {
            0
        };
        own + self
            .children
            .iter()
            .map(|c| c.element_count())
            .sum::<usize>()
    }

    /// Find a node by ID (recursive)
    pub fn find(&self, id: EntityId) -> Option<&SpatialNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a node by ID (mutable, recursive)
    pub fn find_mut(&mut self, id: EntityId) -> Option<&mut SpatialNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Iterate all nodes (depth-first)
    pub fn iter(&self) -> SpatialNodeIter<'_> {
        SpatialNodeIter { stack: vec![self] }
    }

    /// Get all element IDs in this subtree
    pub fn element_ids(&self) -> Vec<EntityId> {
        self.iter()
            .filter(|n| n.node_type == SpatialNodeType::Element)
            .map(|n| n.id)
            .collect()
    }
}

/// Iterator over spatial nodes (depth-first)
pub struct SpatialNodeIter<'a> {
    stack: Vec<&'a SpatialNode>,
}

impl<'a> Iterator for SpatialNodeIter<'a> {
    type Item = &'a SpatialNode;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        // Add children in reverse order so first child is processed first
        for child in node.children.iter().rev() {
            self.stack.push(child);
        }
        Some(node)
    }
}

/// Building storey information
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoreyInfo {
    /// Entity ID
    pub id: EntityId,
    /// Storey name
    pub name: String,
    /// Elevation in meters
    pub elevation: f32,
    /// Number of elements in this storey
    pub element_count: usize,
}

impl StoreyInfo {
    /// Create new storey info
    pub fn new(
        id: EntityId,
        name: impl Into<String>,
        elevation: f32,
        element_count: usize,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            elevation,
            element_count,
        }
    }
}

/// Spatial query interface
///
/// Provides access to the spatial structure hierarchy and search capabilities.
///
/// # Example
///
/// ```ignore
/// use ifc_lite_model::{SpatialQuery, EntityId};
///
/// fn explore_building(spatial: &dyn SpatialQuery) {
///     // Get spatial tree
///     if let Some(tree) = spatial.spatial_tree() {
///         println!("Project: {}", tree.name);
///         for child in &tree.children {
///             println!("  {}: {}", child.node_type.display_name(), child.name);
///         }
///     }
///
///     // List storeys
///     for storey in spatial.storeys() {
///         println!("Storey {} at elevation {}m ({} elements)",
///             storey.name, storey.elevation, storey.element_count);
///     }
///
///     // Search for walls
///     let wall_ids = spatial.search("wall");
///     println!("Found {} walls", wall_ids.len());
/// }
/// ```
pub trait SpatialQuery: Send + Sync {
    /// Get the spatial hierarchy tree
    ///
    /// Returns the root of the spatial structure tree (typically IfcProject).
    /// The tree contains all spatial structure elements and their contained elements.
    ///
    /// # Returns
    /// The root spatial node, or `None` if no spatial structure exists
    fn spatial_tree(&self) -> Option<&SpatialNode>;

    /// Get all building storeys
    ///
    /// Returns information about all storeys in the model, sorted by elevation.
    ///
    /// # Returns
    /// A vector of storey information
    fn storeys(&self) -> Vec<StoreyInfo>;

    /// Get elements contained in a storey
    ///
    /// # Arguments
    /// * `storey_id` - The storey entity ID
    ///
    /// # Returns
    /// A vector of element IDs contained in the storey
    fn elements_in_storey(&self, storey_id: EntityId) -> Vec<EntityId>;

    /// Get the containing storey for an element
    ///
    /// # Arguments
    /// * `element_id` - The element entity ID
    ///
    /// # Returns
    /// The storey ID if the element is contained in a storey
    fn containing_storey(&self, element_id: EntityId) -> Option<EntityId>;

    /// Search entities by name or type
    ///
    /// Performs a case-insensitive search across entity names and types.
    ///
    /// # Arguments
    /// * `query` - The search query string
    ///
    /// # Returns
    /// A vector of matching entity IDs
    fn search(&self, query: &str) -> Vec<EntityId>;

    /// Get elements of a specific type
    ///
    /// # Arguments
    /// * `ifc_type` - The IFC type to filter by
    ///
    /// # Returns
    /// A vector of entity IDs of the specified type
    fn elements_by_type(&self, ifc_type: &IfcType) -> Vec<EntityId>;

    /// Get all building elements (walls, slabs, etc.)
    ///
    /// # Returns
    /// A vector of all building element IDs
    fn all_elements(&self) -> Vec<EntityId> {
        if let Some(tree) = self.spatial_tree() {
            tree.element_ids()
        } else {
            Vec::new()
        }
    }

    /// Get element count
    fn element_count(&self) -> usize {
        self.all_elements().len()
    }
}
