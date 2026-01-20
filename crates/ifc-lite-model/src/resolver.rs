// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Entity resolution trait for looking up and resolving IFC entities

use crate::{AttributeValue, DecodedEntity, EntityId, IfcType};
use std::sync::Arc;

/// Entity lookup and reference resolution
///
/// This trait provides the core functionality for accessing IFC entities
/// and resolving entity references. Implementations should provide O(1)
/// lookup by entity ID.
///
/// # Example
///
/// ```ignore
/// use ifc_lite_model::{EntityResolver, EntityId, AttributeValue};
///
/// fn process_wall(resolver: &dyn EntityResolver, wall_id: EntityId) {
///     if let Some(wall) = resolver.get(wall_id) {
///         println!("Wall type: {:?}", wall.ifc_type);
///
///         // Resolve a reference attribute
///         if let Some(ref_attr) = wall.get(5) {
///             if let Some(related) = resolver.resolve_ref(ref_attr) {
///                 println!("Related entity: {:?}", related.ifc_type);
///             }
///         }
///     }
/// }
/// ```
pub trait EntityResolver: Send + Sync {
    /// Get entity by ID
    ///
    /// Returns the decoded entity if it exists, wrapped in an Arc for
    /// efficient sharing.
    ///
    /// # Arguments
    /// * `id` - The entity ID to look up
    ///
    /// # Returns
    /// `Some(Arc<DecodedEntity>)` if found, `None` otherwise
    fn get(&self, id: EntityId) -> Option<Arc<DecodedEntity>>;

    /// Resolve an entity reference from an attribute value
    ///
    /// If the attribute value is an EntityRef, this looks up and returns
    /// the referenced entity.
    ///
    /// # Arguments
    /// * `attr` - The attribute value that may contain an entity reference
    ///
    /// # Returns
    /// `Some(Arc<DecodedEntity>)` if the attribute is a valid reference, `None` otherwise
    fn resolve_ref(&self, attr: &AttributeValue) -> Option<Arc<DecodedEntity>> {
        match attr {
            AttributeValue::EntityRef(id) => self.get(*id),
            _ => None,
        }
    }

    /// Resolve a list of entity references
    ///
    /// If the attribute value is a List containing EntityRefs, this resolves
    /// all of them and returns the entities.
    ///
    /// # Arguments
    /// * `attr` - The attribute value that may contain a list of entity references
    ///
    /// # Returns
    /// A vector of resolved entities (empty if the attribute is not a list or contains no refs)
    fn resolve_ref_list(&self, attr: &AttributeValue) -> Vec<Arc<DecodedEntity>> {
        match attr {
            AttributeValue::List(items) => items
                .iter()
                .filter_map(|item| self.resolve_ref(item))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Get all entities of a specific type
    ///
    /// # Arguments
    /// * `ifc_type` - The IFC type to filter by
    ///
    /// # Returns
    /// A vector of all entities matching the specified type
    fn entities_by_type(&self, ifc_type: &IfcType) -> Vec<Arc<DecodedEntity>>;

    /// Find entities by type name string
    ///
    /// This is useful for dynamic lookups where the type is not known at compile time.
    ///
    /// # Arguments
    /// * `type_name` - The type name to search for (case-insensitive)
    ///
    /// # Returns
    /// A vector of entities matching the type name
    fn find_by_type_name(&self, type_name: &str) -> Vec<Arc<DecodedEntity>>;

    /// Count entities of a specific type
    ///
    /// # Arguments
    /// * `ifc_type` - The IFC type to count
    ///
    /// # Returns
    /// The number of entities of the specified type
    fn count_by_type(&self, ifc_type: &IfcType) -> usize;

    /// Get all entity IDs in the model
    ///
    /// # Returns
    /// A vector of all entity IDs
    fn all_ids(&self) -> Vec<EntityId>;

    /// Get total entity count
    fn entity_count(&self) -> usize {
        self.all_ids().len()
    }

    /// Fast raw bytes access for optimized parsing
    ///
    /// Returns the raw bytes of an entity's definition for parsers that
    /// want to do direct parsing without going through the attribute system.
    /// This is useful for performance-critical paths like coordinate parsing.
    ///
    /// # Arguments
    /// * `id` - The entity ID to get raw bytes for
    ///
    /// # Returns
    /// The raw bytes of the entity definition, or `None` if not available
    fn raw_bytes(&self, id: EntityId) -> Option<&[u8]>;
}

/// Extension methods for EntityResolver
pub trait EntityResolverExt: EntityResolver {
    /// Get entity by raw u32 ID
    fn get_by_u32(&self, id: u32) -> Option<Arc<DecodedEntity>> {
        self.get(EntityId(id))
    }

    /// Check if an entity exists
    fn exists(&self, id: EntityId) -> bool {
        self.get(id).is_some()
    }

    /// Get entity or return error
    fn get_or_err(&self, id: EntityId) -> crate::Result<Arc<DecodedEntity>> {
        self.get(id).ok_or(crate::ParseError::EntityNotFound(id))
    }

    /// Resolve reference or return error
    fn resolve_ref_or_err(
        &self,
        entity_id: EntityId,
        attr_index: usize,
        attr: &AttributeValue,
    ) -> crate::Result<Arc<DecodedEntity>> {
        self.resolve_ref(attr)
            .ok_or(crate::ParseError::InvalidReference {
                entity: entity_id,
                attribute: attr_index,
            })
    }
}

// Blanket implementation for all EntityResolver types
impl<T: EntityResolver + ?Sized> EntityResolverExt for T {}
