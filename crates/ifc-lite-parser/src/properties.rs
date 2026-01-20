// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! PropertyReader trait implementation

use ifc_lite_model::{
    AttributeValue, DecodedEntity, EntityId, EntityResolver, IfcType, Property, PropertyReader,
    PropertySet, Quantity, QuantityType,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Property reader implementation
pub struct PropertyReaderImpl {
    /// Reference to resolver for entity lookups
    resolver: Arc<dyn EntityResolver>,
    /// Cache: entity ID -> property set IDs
    pset_cache: FxHashMap<u32, Vec<EntityId>>,
    /// Cache: entity ID -> quantity set IDs
    qset_cache: FxHashMap<u32, Vec<EntityId>>,
}

impl PropertyReaderImpl {
    /// Create a new property reader
    pub fn new(resolver: Arc<dyn EntityResolver>) -> Self {
        // Build property relationship cache
        let mut pset_cache: FxHashMap<u32, Vec<EntityId>> = FxHashMap::default();
        let mut qset_cache: FxHashMap<u32, Vec<EntityId>> = FxHashMap::default();

        // Find all IFCRELDEFINESBYPROPERTIES relationships
        for rel in resolver.entities_by_type(&IfcType::IfcRelDefinesByProperties) {
            // RelatedObjects at index 4, RelatingPropertyDefinition at index 5
            let related_ids = match rel.get(4) {
                Some(AttributeValue::List(list)) => list
                    .iter()
                    .filter_map(|v| v.as_entity_ref())
                    .collect::<Vec<_>>(),
                _ => continue,
            };

            let pset_id = match rel.get_ref(5) {
                Some(id) => id,
                None => continue,
            };

            // Check if it's a property set or element quantity
            if let Some(pset) = resolver.get(pset_id) {
                let cache = match pset.ifc_type {
                    IfcType::IfcPropertySet => &mut pset_cache,
                    IfcType::IfcElementQuantity => &mut qset_cache,
                    _ => continue,
                };

                for related_id in related_ids {
                    cache.entry(related_id.0).or_default().push(pset_id);
                }
            }
        }

        Self {
            resolver,
            pset_cache,
            qset_cache,
        }
    }

    /// Extract properties from a property set entity
    fn extract_properties(&self, pset: &DecodedEntity) -> Vec<Property> {
        let mut properties = Vec::new();

        // HasProperties at index 4
        let prop_refs = match pset.get(4) {
            Some(AttributeValue::List(list)) => list,
            _ => return properties,
        };

        for prop_ref in prop_refs {
            if let AttributeValue::EntityRef(prop_id) = prop_ref {
                if let Some(prop_entity) = self.resolver.get(*prop_id) {
                    if let Some(prop) = self.extract_single_property(&prop_entity) {
                        properties.push(prop);
                    }
                }
            }
        }

        properties
    }

    /// Extract a single property from an IfcProperty entity
    fn extract_single_property(&self, prop: &DecodedEntity) -> Option<Property> {
        // Name at index 0
        let name = prop.get_string(0)?.to_string();

        match prop.ifc_type {
            IfcType::IfcPropertySingleValue => {
                // NominalValue at index 2, Unit at index 3
                let value = self.format_value(prop.get(2)?);
                let unit = prop.get(3).and_then(|v| self.extract_unit(v));
                Some(Property {
                    name,
                    value,
                    unit,
                })
            }
            IfcType::IfcPropertyEnumeratedValue => {
                // EnumerationValues at index 2
                let values = match prop.get(2) {
                    Some(AttributeValue::List(list)) => list
                        .iter()
                        .map(|v| self.format_value(v))
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => return None,
                };
                Some(Property {
                    name,
                    value: values,
                    unit: None,
                })
            }
            IfcType::IfcPropertyBoundedValue => {
                // UpperBoundValue, LowerBoundValue at indices 2, 3
                let upper = prop.get(2).map(|v| self.format_value(v));
                let lower = prop.get(3).map(|v| self.format_value(v));
                let value = match (lower, upper) {
                    (Some(l), Some(u)) => format!("{} - {}", l, u),
                    (Some(l), None) => format!(">= {}", l),
                    (None, Some(u)) => format!("<= {}", u),
                    _ => return None,
                };
                Some(Property {
                    name,
                    value,
                    unit: None,
                })
            }
            IfcType::IfcPropertyListValue => {
                // ListValues at index 2
                let values = match prop.get(2) {
                    Some(AttributeValue::List(list)) => list
                        .iter()
                        .map(|v| self.format_value(v))
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => return None,
                };
                Some(Property {
                    name,
                    value: values,
                    unit: None,
                })
            }
            _ => None,
        }
    }

    /// Format an attribute value as a string
    fn format_value(&self, attr: &AttributeValue) -> String {
        match attr {
            AttributeValue::String(s) => s.clone(),
            AttributeValue::Integer(i) => i.to_string(),
            AttributeValue::Float(f) => format!("{:.6}", f).trim_end_matches('0').trim_end_matches('.').to_string(),
            AttributeValue::Bool(b) => b.to_string(),
            AttributeValue::Enum(e) => e.clone(),
            AttributeValue::TypedValue(_, args) if !args.is_empty() => self.format_value(&args[0]),
            AttributeValue::Null => "".to_string(),
            _ => format!("{:?}", attr),
        }
    }

    /// Extract unit from a unit reference
    fn extract_unit(&self, attr: &AttributeValue) -> Option<String> {
        let unit_id = attr.as_entity_ref()?;
        let unit = self.resolver.get(unit_id)?;

        // Try to get a readable unit name
        match unit.ifc_type {
            IfcType::IfcSIUnit => {
                let prefix = unit.get_enum(2).unwrap_or("");
                let name = unit.get_enum(3)?;
                let prefix_str = match prefix {
                    "MILLI" => "m",
                    "CENTI" => "c",
                    "KILO" => "k",
                    _ => "",
                };
                let unit_str = match name {
                    "METRE" => "m",
                    "SQUARE_METRE" => "m²",
                    "CUBIC_METRE" => "m³",
                    "GRAM" => "g",
                    "SECOND" => "s",
                    "KELVIN" => "K",
                    "AMPERE" => "A",
                    _ => name,
                };
                Some(format!("{}{}", prefix_str, unit_str))
            }
            IfcType::IfcConversionBasedUnit => {
                // Name at index 2
                unit.get_string(2).map(|s| s.to_string())
            }
            _ => None,
        }
    }

    /// Extract quantities from an element quantity entity
    fn extract_quantities(&self, qset: &DecodedEntity) -> Vec<Quantity> {
        let mut quantities = Vec::new();

        // Quantities at index 5
        let qty_refs = match qset.get(5) {
            Some(AttributeValue::List(list)) => list,
            _ => return quantities,
        };

        for qty_ref in qty_refs {
            if let AttributeValue::EntityRef(qty_id) = qty_ref {
                if let Some(qty_entity) = self.resolver.get(*qty_id) {
                    if let Some(qty) = self.extract_single_quantity(&qty_entity) {
                        quantities.push(qty);
                    }
                }
            }
        }

        quantities
    }

    /// Extract a single quantity from an IfcQuantity entity
    fn extract_single_quantity(&self, qty: &DecodedEntity) -> Option<Quantity> {
        // Name at index 0
        let name = qty.get_string(0)?.to_string();

        let (value, quantity_type) = match qty.ifc_type {
            IfcType::IfcQuantityLength => (qty.get_float(3)?, QuantityType::Length),
            IfcType::IfcQuantityArea => (qty.get_float(3)?, QuantityType::Area),
            IfcType::IfcQuantityVolume => (qty.get_float(3)?, QuantityType::Volume),
            IfcType::IfcQuantityCount => (qty.get_float(3)?, QuantityType::Count),
            IfcType::IfcQuantityWeight => (qty.get_float(3)?, QuantityType::Weight),
            IfcType::IfcQuantityTime => (qty.get_float(3)?, QuantityType::Time),
            _ => return None,
        };

        Some(Quantity::new(name, value, quantity_type))
    }
}

impl PropertyReader for PropertyReaderImpl {
    fn property_sets(&self, id: EntityId) -> Vec<PropertySet> {
        let pset_ids = match self.pset_cache.get(&id.0) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        let mut result = Vec::new();

        for pset_id in pset_ids {
            if let Some(pset) = self.resolver.get(*pset_id) {
                // Name at index 2
                let name = pset.get_string(2).unwrap_or("Unknown").to_string();
                let properties = self.extract_properties(&pset);

                if !properties.is_empty() {
                    result.push(PropertySet { name, properties });
                }
            }
        }

        result
    }

    fn quantities(&self, id: EntityId) -> Vec<Quantity> {
        let qset_ids = match self.qset_cache.get(&id.0) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        let mut result = Vec::new();

        for qset_id in qset_ids {
            if let Some(qset) = self.resolver.get(*qset_id) {
                result.extend(self.extract_quantities(&qset));
            }
        }

        result
    }

    fn global_id(&self, id: EntityId) -> Option<String> {
        let entity = self.resolver.get(id)?;
        // GlobalId is typically at index 0 for most entities
        entity.get_string(0).map(|s| s.to_string())
    }

    fn name(&self, id: EntityId) -> Option<String> {
        let entity = self.resolver.get(id)?;
        // Name is typically at index 2 for most building elements
        entity.get_string(2).map(|s| s.to_string())
    }

    fn description(&self, id: EntityId) -> Option<String> {
        let entity = self.resolver.get(id)?;
        // Description is typically at index 3
        entity.get_string(3).map(|s| s.to_string())
    }

    fn object_type(&self, id: EntityId) -> Option<String> {
        let entity = self.resolver.get(id)?;
        // ObjectType is typically at index 4
        entity.get_string(4).map(|s| s.to_string())
    }

    fn tag(&self, id: EntityId) -> Option<String> {
        let entity = self.resolver.get(id)?;
        // Tag varies by entity type, usually at index 7 for building elements
        entity.get_string(7).map(|s| s.to_string())
    }
}
