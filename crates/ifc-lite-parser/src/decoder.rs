// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Lazy entity decoder with caching

use crate::scanner::{EntityIndex, EntityScanner};
use crate::tokenizer::parse_entity_at;
use ifc_lite_model::{DecodedEntity, EntityId, ParseError, Result};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Lazy entity decoder with caching
///
/// Decodes entities on-demand and caches them for reuse. Uses an index
/// for O(1) entity lookup by ID.
pub struct EntityDecoder<'a> {
    /// Raw IFC content
    content: &'a str,
    /// Entity ID -> (start, end) byte offsets
    index: EntityIndex,
    /// Decoded entity cache
    cache: FxHashMap<u32, Arc<DecodedEntity>>,
    /// Cached length unit scale
    unit_scale: Option<f64>,
}

impl<'a> EntityDecoder<'a> {
    /// Create a new decoder for the given content
    pub fn new(content: &'a str) -> Self {
        let index = EntityScanner::build_index(content);

        Self {
            content,
            index,
            cache: FxHashMap::default(),
            unit_scale: None,
        }
    }

    /// Create decoder with pre-built index
    pub fn with_index(content: &'a str, index: EntityIndex) -> Self {
        Self {
            content,
            index,
            cache: FxHashMap::default(),
            unit_scale: None,
        }
    }

    /// Get raw content
    pub fn content(&self) -> &'a str {
        self.content
    }

    /// Get entity index
    pub fn index(&self) -> &EntityIndex {
        &self.index
    }

    /// Get all entity IDs
    pub fn all_ids(&self) -> Vec<EntityId> {
        self.index.keys().map(|&id| EntityId(id)).collect()
    }

    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.index.len()
    }

    /// Check if entity exists
    pub fn exists(&self, id: EntityId) -> bool {
        self.index.contains_key(&id.0)
    }

    /// Decode entity by ID
    pub fn decode_by_id(&mut self, id: EntityId) -> Result<Arc<DecodedEntity>> {
        // Check cache first
        if let Some(cached) = self.cache.get(&id.0) {
            return Ok(Arc::clone(cached));
        }

        // Get byte offsets
        let (start, end) = self
            .index
            .get(&id.0)
            .ok_or(ParseError::EntityNotFound(id))?;

        // Parse entity
        let entity = parse_entity_at(self.content, *start, *end)
            .map_err(|e| ParseError::EntityParse(id, e))?;

        // Cache and return
        let arc = Arc::new(entity);
        self.cache.insert(id.0, Arc::clone(&arc));
        Ok(arc)
    }

    /// Get raw bytes for an entity (for fast parsing)
    pub fn raw_bytes(&self, id: EntityId) -> Option<&'a [u8]> {
        let (start, end) = self.index.get(&id.0)?;
        Some(self.content[*start..*end].as_bytes())
    }

    /// Get raw string for an entity
    pub fn raw_str(&self, id: EntityId) -> Option<&'a str> {
        let (start, end) = self.index.get(&id.0)?;
        Some(&self.content[*start..*end])
    }

    /// Get cached unit scale
    pub fn unit_scale(&self) -> Option<f64> {
        self.unit_scale
    }

    /// Set unit scale (called after extraction)
    pub fn set_unit_scale(&mut self, scale: f64) {
        self.unit_scale = Some(scale);
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Pre-warm cache with specific entities
    pub fn preload(&mut self, ids: &[EntityId]) {
        for id in ids {
            let _ = self.decode_by_id(*id);
        }
    }

    /// Find entities by type name
    pub fn find_by_type(&mut self, type_name: &str) -> Vec<Arc<DecodedEntity>> {
        let matches = EntityScanner::find_by_type(self.content, type_name);
        let mut results = Vec::with_capacity(matches.len());

        for (id, _, _) in matches {
            if let Ok(entity) = self.decode_by_id(EntityId(id)) {
                results.push(entity);
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_IFC: &str = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCPROJECT('guid',$,'Project',$,$,$,$,$,#2);
#2=IFCUNITASSIGNMENT((#3));
#3=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
ENDSEC;
END-ISO-10303-21;
"#;

    #[test]
    fn test_decode_by_id() {
        let mut decoder = EntityDecoder::new(TEST_IFC);

        let entity = decoder.decode_by_id(EntityId(1)).unwrap();
        assert_eq!(entity.id, EntityId(1));
        assert_eq!(entity.ifc_type, ifc_lite_model::IfcType::IfcProject);
    }

    #[test]
    fn test_caching() {
        let mut decoder = EntityDecoder::new(TEST_IFC);

        // First decode
        let entity1 = decoder.decode_by_id(EntityId(1)).unwrap();
        assert_eq!(decoder.cache_size(), 1);

        // Second decode should return cached
        let entity2 = decoder.decode_by_id(EntityId(1)).unwrap();
        assert_eq!(decoder.cache_size(), 1);

        // Should be same Arc
        assert!(Arc::ptr_eq(&entity1, &entity2));
    }

    #[test]
    fn test_entity_not_found() {
        let mut decoder = EntityDecoder::new(TEST_IFC);

        let result = decoder.decode_by_id(EntityId(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_find_by_type() {
        let mut decoder = EntityDecoder::new(TEST_IFC);

        let projects = decoder.find_by_type("IFCPROJECT");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, EntityId(1));
    }
}
