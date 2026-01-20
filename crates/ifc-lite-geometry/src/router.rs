// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Geometry Router - Dynamic dispatch to geometry processors
//!
//! Routes IFC representation entities to appropriate processors based on type.
//! Uses the `EntityResolver` trait from ifc-lite-model for entity lookup.

use crate::{Error, Mesh, Result};
use ifc_lite_model::{DecodedEntity, EntityId, EntityResolver, IfcType};
use nalgebra::Matrix4;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Geometry processor trait
///
/// Each processor handles one or more types of IFC geometry representations.
/// Processors use the `EntityResolver` trait for entity lookups, making them
/// independent of any specific parser implementation.
pub trait GeometryProcessor: Send + Sync {
    /// Process entity into mesh
    ///
    /// # Arguments
    /// * `entity` - The decoded IFC entity to process
    /// * `resolver` - Entity resolver for looking up referenced entities
    /// * `unit_scale` - Scale factor from file units to meters
    ///
    /// # Returns
    /// The processed mesh, or an error if processing fails
    fn process(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
        unit_scale: f64,
    ) -> Result<Mesh>;

    /// Get supported IFC types
    fn supported_types(&self) -> Vec<IfcType>;
}

/// Geometry router - routes entities to processors
///
/// The router dispatches IFC geometry entities to appropriate processors based
/// on their type. It also manages caching for instanced geometry (MappedItems)
/// and geometry deduplication.
pub struct GeometryRouter {
    /// Registered processors by type
    processors: HashMap<IfcType, Arc<dyn GeometryProcessor>>,
    /// Cache for IfcRepresentationMap source geometry (MappedItem instancing)
    mapped_item_cache: RefCell<FxHashMap<u32, Arc<Mesh>>>,
    /// Cache for FacetedBrep geometry (batch processed)
    faceted_brep_cache: RefCell<FxHashMap<u32, Mesh>>,
    /// Cache for geometry deduplication by content hash
    geometry_hash_cache: RefCell<FxHashMap<u64, Arc<Mesh>>>,
    /// Unit scale factor (e.g., 0.001 for millimeters -> meters)
    unit_scale: f64,
}

impl GeometryRouter {
    /// Create new router without any processors registered
    pub fn new() -> Self {
        Self {
            processors: HashMap::new(),
            mapped_item_cache: RefCell::new(FxHashMap::default()),
            faceted_brep_cache: RefCell::new(FxHashMap::default()),
            geometry_hash_cache: RefCell::new(FxHashMap::default()),
            unit_scale: 1.0,
        }
    }

    /// Create router with default processors registered
    ///
    /// Registers the following processors:
    /// - `ExtrudedAreaSolidProcessor` (IfcExtrudedAreaSolid)
    /// - `TriangulatedFaceSetProcessor` (IfcTriangulatedFaceSet)
    /// - `FacetedBrepProcessor` (IfcFacetedBrep)
    /// - `SweptDiskSolidProcessor` (IfcSweptDiskSolid)
    /// - `RevolvedAreaSolidProcessor` (IfcRevolvedAreaSolid)
    pub fn with_default_processors() -> Self {
        use crate::processors::{
            ExtrudedAreaSolidProcessor, FacetedBrepProcessor, RevolvedAreaSolidProcessor,
            SweptDiskSolidProcessor, TriangulatedFaceSetProcessor,
        };

        let mut router = Self::new();
        router.register(Arc::new(ExtrudedAreaSolidProcessor::new()));
        router.register(Arc::new(TriangulatedFaceSetProcessor::new()));
        router.register(Arc::new(FacetedBrepProcessor::new()));
        router.register(Arc::new(SweptDiskSolidProcessor::new()));
        router.register(Arc::new(RevolvedAreaSolidProcessor::new()));
        router
    }

    /// Create router with specific unit scale (without default processors)
    pub fn with_unit_scale(unit_scale: f64) -> Self {
        let mut router = Self::new();
        router.unit_scale = unit_scale;
        router
    }

    /// Create router with default processors and specific unit scale
    pub fn with_default_processors_and_unit_scale(unit_scale: f64) -> Self {
        let mut router = Self::with_default_processors();
        router.unit_scale = unit_scale;
        router
    }

    /// Get the current unit scale factor
    pub fn unit_scale(&self) -> f64 {
        self.unit_scale
    }

    /// Set the unit scale factor
    pub fn set_unit_scale(&mut self, scale: f64) {
        self.unit_scale = scale;
    }

    /// Register a geometry processor
    pub fn register(&mut self, processor: Arc<dyn GeometryProcessor>) {
        for ifc_type in processor.supported_types() {
            self.processors.insert(ifc_type, Arc::clone(&processor));
        }
    }

    /// Check if a type has a registered processor
    pub fn has_processor(&self, ifc_type: &IfcType) -> bool {
        self.processors.contains_key(ifc_type)
    }

    /// Process a single representation item
    pub fn process_representation_item(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Result<Mesh> {
        // Check for cached mapped item
        if entity.ifc_type == IfcType::IfcMappedItem {
            if let Some(cached) = self.get_cached_mapped_item(entity.id.0) {
                let mut mesh = (*cached).clone();
                // Apply MappingTarget transform
                if let Some(transform) = self.extract_mapping_target_transform(entity, resolver) {
                    crate::extrusion::apply_transform(&mut mesh, &transform);
                }
                self.scale_mesh(&mut mesh);
                return Ok(mesh);
            }
        }

        // Check for cached faceted brep
        if entity.ifc_type == IfcType::IfcFacetedBrep {
            if let Some(cached) = self.take_cached_faceted_brep(entity.id.0) {
                let mut mesh = cached;
                self.scale_mesh(&mut mesh);
                return Ok(mesh);
            }
        }

        // Find and use processor
        let processor = self
            .processors
            .get(&entity.ifc_type)
            .ok_or_else(|| Error::unsupported_type(format!("{:?}", entity.ifc_type)))?;

        let mut mesh = processor.process(entity, resolver, self.unit_scale)?;
        self.scale_mesh(&mut mesh);

        // Cache mapped item source geometry
        if entity.ifc_type == IfcType::IfcMappedItem {
            // Extract and cache the source geometry
            if let Some(source_id) = self.extract_mapping_source_id(entity, resolver) {
                self.cache_mapped_item(source_id, Arc::new(mesh.clone()));
            }
        }

        Ok(mesh)
    }

    /// Process a building element's geometry
    ///
    /// Follows the IFC representation chain:
    /// Element -> Representation -> ShapeRepresentation -> Items
    pub fn process_element(
        &self,
        element: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Result<Mesh> {
        let mut combined_mesh = Mesh::new();

        // Get Representation (typically at index 6 for products)
        let rep_id = match element.get_ref(6) {
            Some(id) => id,
            None => return Ok(combined_mesh), // No representation
        };

        let representation = match resolver.get(rep_id) {
            Some(rep) => rep,
            None => return Ok(combined_mesh),
        };

        // Get Representations list (index 1 in IfcProductDefinitionShape)
        let reps = match representation.get(1) {
            Some(ifc_lite_model::AttributeValue::List(list)) => list,
            _ => return Ok(combined_mesh),
        };

        // Process each shape representation
        for rep_ref in reps {
            if let Some(shape_rep_id) = rep_ref.as_entity_ref() {
                if let Some(shape_rep) = resolver.get(shape_rep_id) {
                    // Filter to Body representations
                    if let Some(rep_type) = shape_rep.get_string(1) {
                        if rep_type != "Body" && rep_type != "Facetation" {
                            continue;
                        }
                    }

                    // Process representation items
                    if let Some(mesh) = self.process_shape_representation(&shape_rep, resolver)? {
                        combined_mesh.merge(&mesh);
                    }
                }
            }
        }

        // Apply object placement transform
        if let Some(placement_id) = element.get_ref(5) {
            if let Some(transform) = self.resolve_placement(placement_id, resolver) {
                crate::extrusion::apply_transform(&mut combined_mesh, &transform);
            }
        }

        Ok(combined_mesh)
    }

    /// Process a shape representation's items
    fn process_shape_representation(
        &self,
        shape_rep: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Result<Option<Mesh>> {
        // Get Items (index 3 in IfcShapeRepresentation)
        let items = match shape_rep.get(3) {
            Some(ifc_lite_model::AttributeValue::List(list)) => list,
            _ => return Ok(None),
        };

        let mut combined = Mesh::new();

        for item_ref in items {
            if let Some(item_id) = item_ref.as_entity_ref() {
                if let Some(item) = resolver.get(item_id) {
                    match self.process_representation_item(&item, resolver) {
                        Ok(mesh) => combined.merge(&mesh),
                        Err(_) => continue, // Skip items that fail to process
                    }
                }
            }
        }

        if combined.is_empty() {
            Ok(None)
        } else {
            Ok(Some(combined))
        }
    }

    /// Scale mesh positions from file units to meters
    #[inline]
    fn scale_mesh(&self, mesh: &mut Mesh) {
        if self.unit_scale != 1.0 {
            let scale = self.unit_scale as f32;
            for pos in mesh.positions.iter_mut() {
                *pos *= scale;
            }
        }
    }

    /// Cache a mapped item's source geometry
    fn cache_mapped_item(&self, source_id: u32, mesh: Arc<Mesh>) {
        self.mapped_item_cache.borrow_mut().insert(source_id, mesh);
    }

    /// Get cached mapped item geometry
    fn get_cached_mapped_item(&self, source_id: u32) -> Option<Arc<Mesh>> {
        self.mapped_item_cache.borrow().get(&source_id).cloned()
    }

    /// Take FacetedBrep from cache (removes entry)
    pub fn take_cached_faceted_brep(&self, brep_id: u32) -> Option<Mesh> {
        self.faceted_brep_cache.borrow_mut().remove(&brep_id)
    }

    /// Cache a faceted brep mesh
    pub fn cache_faceted_brep(&self, brep_id: u32, mesh: Mesh) {
        self.faceted_brep_cache.borrow_mut().insert(brep_id, mesh);
    }

    /// Extract the MappingSource ID from a MappedItem
    fn extract_mapping_source_id(
        &self,
        mapped_item: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Option<u32> {
        // MappingSource at index 0
        let source_id = mapped_item.get_ref(0)?;
        let source = resolver.get(source_id)?;

        // RepresentationMap -> MappedRepresentation (index 1)
        source.get_ref(1).map(|id| id.0)
    }

    /// Extract the MappingTarget transform from a MappedItem
    fn extract_mapping_target_transform(
        &self,
        mapped_item: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Option<Matrix4<f64>> {
        // MappingTarget at index 1
        let target_id = mapped_item.get_ref(1)?;
        self.resolve_placement(target_id, resolver)
    }

    /// Resolve a placement to a transformation matrix
    fn resolve_placement(
        &self,
        placement_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Option<Matrix4<f64>> {
        let placement = resolver.get(placement_id)?;

        match placement.ifc_type {
            IfcType::IfcLocalPlacement => {
                // RelativePlacement at index 1
                let relative_id = placement.get_ref(1)?;
                self.resolve_axis_placement(relative_id, resolver)
            }
            IfcType::IfcAxis2Placement3D => self.resolve_axis_placement(placement_id, resolver),
            IfcType::IfcCartesianTransformationOperator3D
            | IfcType::IfcCartesianTransformationOperator3DnonUniform => {
                self.resolve_transformation_operator(placement_id, resolver)
            }
            _ => None,
        }
    }

    /// Resolve an IfcAxis2Placement3D to a transformation matrix
    fn resolve_axis_placement(
        &self,
        placement_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Option<Matrix4<f64>> {
        let placement = resolver.get(placement_id)?;

        if placement.ifc_type != IfcType::IfcAxis2Placement3D {
            return None;
        }

        // Location (index 0)
        let location = self.resolve_cartesian_point(placement.get_ref(0)?, resolver)?;

        // Axis (index 1) - Z direction, optional
        let axis = placement
            .get_ref(1)
            .and_then(|id| self.resolve_direction(id, resolver))
            .unwrap_or_else(|| nalgebra::Vector3::new(0.0, 0.0, 1.0));

        // RefDirection (index 2) - X direction, optional
        let ref_dir = placement
            .get_ref(2)
            .and_then(|id| self.resolve_direction(id, resolver))
            .unwrap_or_else(|| nalgebra::Vector3::new(1.0, 0.0, 0.0));

        // Build orthonormal basis
        let z = axis.normalize();
        let x = ref_dir.normalize();
        let y = z.cross(&x).normalize();
        let x = y.cross(&z).normalize();

        Some(Matrix4::new(
            x.x, y.x, z.x, location.x, x.y, y.y, z.y, location.y, x.z, y.z, z.z, location.z, 0.0,
            0.0, 0.0, 1.0,
        ))
    }

    /// Resolve a CartesianTransformationOperator3D to a transformation matrix
    fn resolve_transformation_operator(
        &self,
        op_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Option<Matrix4<f64>> {
        let op = resolver.get(op_id)?;

        // LocalOrigin (index 3)
        let origin = self.resolve_cartesian_point(op.get_ref(3)?, resolver)?;

        // Scale (index 6), default 1.0
        let scale = op.get_float(6).unwrap_or(1.0);

        // Build matrix with translation and uniform scale
        let mut matrix = Matrix4::identity();
        matrix[(0, 0)] = scale;
        matrix[(1, 1)] = scale;
        matrix[(2, 2)] = scale;
        matrix[(0, 3)] = origin.x;
        matrix[(1, 3)] = origin.y;
        matrix[(2, 3)] = origin.z;

        Some(matrix)
    }

    /// Resolve an IfcCartesianPoint to a Point3
    fn resolve_cartesian_point(
        &self,
        point_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Option<nalgebra::Point3<f64>> {
        let point = resolver.get(point_id)?;

        if point.ifc_type != IfcType::IfcCartesianPoint {
            return None;
        }

        // Coordinates at index 0
        let coords = point.get(0)?.as_list()?;

        let x = coords.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let y = coords.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
        let z = coords.get(2).and_then(|v| v.as_float()).unwrap_or(0.0);

        Some(nalgebra::Point3::new(x, y, z))
    }

    /// Resolve an IfcDirection to a Vector3
    fn resolve_direction(
        &self,
        dir_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Option<nalgebra::Vector3<f64>> {
        let direction = resolver.get(dir_id)?;

        if direction.ifc_type != IfcType::IfcDirection {
            return None;
        }

        // DirectionRatios at index 0
        let ratios = direction.get(0)?.as_list()?;

        let x = ratios.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let y = ratios.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
        let z = ratios.get(2).and_then(|v| v.as_float()).unwrap_or(1.0);

        Some(nalgebra::Vector3::new(x, y, z))
    }

    /// Compute hash of mesh geometry for deduplication
    #[inline]
    fn compute_mesh_hash(mesh: &Mesh) -> u64 {
        use rustc_hash::FxHasher;
        let mut hasher = FxHasher::default();

        mesh.positions.len().hash(&mut hasher);
        mesh.indices.len().hash(&mut hasher);

        // Hash a sample of positions for speed
        for (i, pos) in mesh.positions.iter().enumerate() {
            if i % 10 == 0 {
                pos.to_bits().hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Try to get deduplicated mesh from cache
    pub fn get_deduplicated(&self, mesh: &Mesh) -> Option<Arc<Mesh>> {
        let hash = Self::compute_mesh_hash(mesh);
        self.geometry_hash_cache.borrow().get(&hash).cloned()
    }

    /// Store mesh in deduplication cache
    pub fn store_deduplicated(&self, mesh: Arc<Mesh>) {
        let hash = Self::compute_mesh_hash(&mesh);
        self.geometry_hash_cache.borrow_mut().insert(hash, mesh);
    }

    /// Clear all caches
    pub fn clear_caches(&self) {
        self.mapped_item_cache.borrow_mut().clear();
        self.faceted_brep_cache.borrow_mut().clear();
        self.geometry_hash_cache.borrow_mut().clear();
    }
}

impl Default for GeometryRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = GeometryRouter::new();
        assert_eq!(router.unit_scale(), 1.0);
        // Empty router has no processors
        assert!(!router.has_processor(&IfcType::IfcExtrudedAreaSolid));
    }

    #[test]
    fn test_router_with_unit_scale() {
        let router = GeometryRouter::with_unit_scale(0.001);
        assert_eq!(router.unit_scale(), 0.001);
    }

    #[test]
    fn test_router_with_default_processors() {
        let router = GeometryRouter::with_default_processors();
        assert_eq!(router.unit_scale(), 1.0);

        // Should have all default processors
        assert!(router.has_processor(&IfcType::IfcExtrudedAreaSolid));
        assert!(router.has_processor(&IfcType::IfcTriangulatedFaceSet));
        assert!(router.has_processor(&IfcType::IfcFacetedBrep));
        assert!(router.has_processor(&IfcType::IfcSweptDiskSolid));
        assert!(router.has_processor(&IfcType::IfcRevolvedAreaSolid));
    }

    #[test]
    fn test_router_with_default_processors_and_unit_scale() {
        let router = GeometryRouter::with_default_processors_and_unit_scale(0.001);
        assert_eq!(router.unit_scale(), 0.001);

        // Should have all default processors
        assert!(router.has_processor(&IfcType::IfcExtrudedAreaSolid));
        assert!(router.has_processor(&IfcType::IfcTriangulatedFaceSet));
    }
}
