// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! EntityResolver trait implementation

use crate::scanner::EntityIndex;
use crate::tokenizer::parse_entity_at;
use ifc_lite_model::{AttributeValue, DecodedEntity, EntityId, EntityResolver, IfcType};
use rustc_hash::FxHashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe entity resolver implementation
pub struct ResolverImpl {
    /// Raw IFC content (owned for thread safety)
    content: String,
    /// Entity ID -> (start, end) byte offsets
    index: EntityIndex,
    /// Decoded entity cache (thread-safe)
    cache: RwLock<FxHashMap<u32, Arc<DecodedEntity>>>,
    /// Type -> entity IDs index
    type_index: FxHashMap<IfcType, Vec<EntityId>>,
}

impl ResolverImpl {
    /// Create a new resolver
    pub fn new(content: String, index: EntityIndex) -> Self {
        // Build type index
        let mut type_index: FxHashMap<IfcType, Vec<EntityId>> = FxHashMap::default();

        // We need to scan entities to build type index
        // This is done during initial parse
        for (&id, (start, end)) in &index {
            if let Ok(entity) = parse_entity_at(&content, *start, *end) {
                type_index
                    .entry(entity.ifc_type.clone())
                    .or_default()
                    .push(EntityId(id));
            }
        }

        Self {
            content,
            index,
            cache: RwLock::new(FxHashMap::default()),
            type_index,
        }
    }

    /// Create resolver with pre-built type index
    pub fn with_type_index(
        content: String,
        index: EntityIndex,
        type_index: FxHashMap<IfcType, Vec<EntityId>>,
    ) -> Self {
        Self {
            content,
            index,
            cache: RwLock::new(FxHashMap::default()),
            type_index,
        }
    }

    /// Get raw content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Decode and cache an entity
    fn decode_and_cache(&self, id: u32) -> Option<Arc<DecodedEntity>> {
        // Check cache with read lock
        {
            let cache = self.cache.read().ok()?;
            if let Some(cached) = cache.get(&id) {
                return Some(Arc::clone(cached));
            }
        }

        // Get byte offsets
        let (start, end) = self.index.get(&id)?;

        // Parse entity
        let entity = parse_entity_at(&self.content, *start, *end).ok()?;
        let arc = Arc::new(entity);

        // Cache with write lock
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(id, Arc::clone(&arc));
        }

        Some(arc)
    }
}

impl EntityResolver for ResolverImpl {
    fn get(&self, id: EntityId) -> Option<Arc<DecodedEntity>> {
        self.decode_and_cache(id.0)
    }

    fn resolve_ref(&self, attr: &AttributeValue) -> Option<Arc<DecodedEntity>> {
        match attr {
            AttributeValue::EntityRef(id) => self.get(*id),
            _ => None,
        }
    }

    fn resolve_ref_list(&self, attr: &AttributeValue) -> Vec<Arc<DecodedEntity>> {
        match attr {
            AttributeValue::List(items) => items
                .iter()
                .filter_map(|item| self.resolve_ref(item))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn entities_by_type(&self, ifc_type: &IfcType) -> Vec<Arc<DecodedEntity>> {
        self.type_index
            .get(ifc_type)
            .map(|ids| ids.iter().filter_map(|id| self.get(*id)).collect())
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
        self.index.keys().map(|&id| EntityId(id)).collect()
    }

    fn raw_bytes(&self, id: EntityId) -> Option<&[u8]> {
        let (start, end) = self.index.get(&id.0)?;
        Some(self.content[*start..*end].as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::EntityScanner;

    const TEST_IFC: &str = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCPROJECT('guid',$,'Project',$,$,$,$,$,#2);
#2=IFCUNITASSIGNMENT((#3));
#3=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#4=IFCWALL('guid2',$,'Wall 1',$,$,$,$,$);
ENDSEC;
END-ISO-10303-21;
"#;

    #[test]
    fn test_resolver_get() {
        let index = EntityScanner::build_index(TEST_IFC);
        let resolver = ResolverImpl::new(TEST_IFC.to_string(), index);

        let entity = resolver.get(EntityId(1)).unwrap();
        assert_eq!(entity.id, EntityId(1));
    }

    #[test]
    fn test_resolver_entities_by_type() {
        let index = EntityScanner::build_index(TEST_IFC);
        let resolver = ResolverImpl::new(TEST_IFC.to_string(), index);

        let walls = resolver.entities_by_type(&IfcType::IfcWall);
        assert_eq!(walls.len(), 1);
        assert_eq!(walls[0].id, EntityId(4));
    }

    #[test]
    fn test_resolver_find_by_type_name() {
        let index = EntityScanner::build_index(TEST_IFC);
        let resolver = ResolverImpl::new(TEST_IFC.to_string(), index);

        let projects = resolver.find_by_type_name("IFCPROJECT");
        assert_eq!(projects.len(), 1);
    }

    #[test]
    fn test_resolver_thread_safe() {
        use std::thread;

        let index = EntityScanner::build_index(TEST_IFC);
        let resolver = Arc::new(ResolverImpl::new(TEST_IFC.to_string(), index));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let resolver = Arc::clone(&resolver);
                thread::spawn(move || {
                    for id in 1..=4 {
                        let _ = resolver.get(EntityId(id));
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
