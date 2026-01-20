// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Property and quantity access for IFC entities

use crate::EntityId;
use serde::{Deserialize, Serialize};

/// A single property value with optional unit
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Property {
    /// Property name
    pub name: String,
    /// Property value as formatted string
    pub value: String,
    /// Unit of measurement (if applicable)
    pub unit: Option<String>,
}

impl Property {
    /// Create a new property
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            unit: None,
        }
    }

    /// Create a property with unit
    pub fn with_unit(
        name: impl Into<String>,
        value: impl Into<String>,
        unit: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            unit: Some(unit.into()),
        }
    }
}

/// A property set containing multiple properties
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertySet {
    /// Property set name (e.g., "Pset_WallCommon")
    pub name: String,
    /// Properties in this set
    pub properties: Vec<Property>,
}

impl PropertySet {
    /// Create a new property set
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: Vec::new(),
        }
    }

    /// Add a property to this set
    pub fn add(&mut self, property: Property) {
        self.properties.push(property);
    }

    /// Get a property by name
    pub fn get(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
    }
}

/// Quantity types supported in IFC
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuantityType {
    /// Linear measurement (IfcQuantityLength)
    Length,
    /// Area measurement (IfcQuantityArea)
    Area,
    /// Volume measurement (IfcQuantityVolume)
    Volume,
    /// Count (IfcQuantityCount)
    Count,
    /// Weight/mass measurement (IfcQuantityWeight)
    Weight,
    /// Time measurement (IfcQuantityTime)
    Time,
}

impl QuantityType {
    /// Get default unit for this quantity type
    pub fn default_unit(&self) -> &'static str {
        match self {
            QuantityType::Length => "m",
            QuantityType::Area => "m²",
            QuantityType::Volume => "m³",
            QuantityType::Count => "",
            QuantityType::Weight => "kg",
            QuantityType::Time => "s",
        }
    }
}

/// A quantity value with type and unit
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    /// Quantity name
    pub name: String,
    /// Numeric value
    pub value: f64,
    /// Unit of measurement
    pub unit: String,
    /// Type of quantity
    pub quantity_type: QuantityType,
}

impl Quantity {
    /// Create a new quantity
    pub fn new(name: impl Into<String>, value: f64, quantity_type: QuantityType) -> Self {
        Self {
            name: name.into(),
            value,
            unit: quantity_type.default_unit().to_string(),
            quantity_type,
        }
    }

    /// Create a quantity with custom unit
    pub fn with_unit(
        name: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
        quantity_type: QuantityType,
    ) -> Self {
        Self {
            name: name.into(),
            value,
            unit: unit.into(),
            quantity_type,
        }
    }

    /// Format the value with unit
    pub fn formatted(&self) -> String {
        if self.unit.is_empty() {
            format!("{}", self.value)
        } else {
            format!("{} {}", self.value, self.unit)
        }
    }
}

/// Property and quantity reader trait
///
/// Provides access to property sets and quantities associated with IFC entities.
/// Property sets come from IfcPropertySet entities linked via IfcRelDefinesByProperties.
/// Quantities come from IfcElementQuantity entities.
///
/// # Example
///
/// ```ignore
/// use ifc_lite_model::{PropertyReader, EntityId};
///
/// fn show_wall_properties(props: &dyn PropertyReader, wall_id: EntityId) {
///     // Get all property sets
///     for pset in props.property_sets(wall_id) {
///         println!("Property Set: {}", pset.name);
///         for prop in &pset.properties {
///             println!("  {}: {}", prop.name, prop.value);
///         }
///     }
///
///     // Get quantities
///     for qty in props.quantities(wall_id) {
///         println!("{}: {} {}", qty.name, qty.value, qty.unit);
///     }
///
///     // Get specific property
///     if let Some(fire_rating) = props.get_property(wall_id, "FireRating") {
///         println!("Fire Rating: {}", fire_rating.value);
///     }
/// }
/// ```
pub trait PropertyReader: Send + Sync {
    /// Get all property sets associated with an entity
    ///
    /// # Arguments
    /// * `id` - The entity ID to get property sets for
    ///
    /// # Returns
    /// A vector of property sets (empty if none found)
    fn property_sets(&self, id: EntityId) -> Vec<PropertySet>;

    /// Get all quantities associated with an entity
    ///
    /// # Arguments
    /// * `id` - The entity ID to get quantities for
    ///
    /// # Returns
    /// A vector of quantities (empty if none found)
    fn quantities(&self, id: EntityId) -> Vec<Quantity>;

    /// Get a specific property by name
    ///
    /// Searches all property sets for the entity and returns the first
    /// property with the matching name.
    ///
    /// # Arguments
    /// * `id` - The entity ID to search
    /// * `name` - The property name to find
    ///
    /// # Returns
    /// The property if found
    fn get_property(&self, id: EntityId, name: &str) -> Option<Property> {
        self.property_sets(id)
            .into_iter()
            .flat_map(|pset| pset.properties)
            .find(|p| p.name == name)
    }

    /// Get a specific quantity by name
    ///
    /// # Arguments
    /// * `id` - The entity ID to search
    /// * `name` - The quantity name to find
    ///
    /// # Returns
    /// The quantity if found
    fn get_quantity(&self, id: EntityId, name: &str) -> Option<Quantity> {
        self.quantities(id).into_iter().find(|q| q.name == name)
    }

    /// Get entity's GlobalId (GUID)
    ///
    /// The GlobalId is a unique identifier assigned to IFC entities,
    /// typically a 22-character base64-encoded GUID.
    ///
    /// # Arguments
    /// * `id` - The entity ID
    ///
    /// # Returns
    /// The GlobalId string if available
    fn global_id(&self, id: EntityId) -> Option<String>;

    /// Get entity's Name attribute
    ///
    /// Most IFC entities have a Name attribute (typically at index 2).
    ///
    /// # Arguments
    /// * `id` - The entity ID
    ///
    /// # Returns
    /// The name string if available
    fn name(&self, id: EntityId) -> Option<String>;

    /// Get entity's Description attribute
    ///
    /// # Arguments
    /// * `id` - The entity ID
    ///
    /// # Returns
    /// The description string if available
    fn description(&self, id: EntityId) -> Option<String>;

    /// Get entity's ObjectType attribute
    ///
    /// ObjectType is often used as a more specific type indicator
    /// beyond the IFC class name.
    ///
    /// # Arguments
    /// * `id` - The entity ID
    ///
    /// # Returns
    /// The object type string if available
    fn object_type(&self, _id: EntityId) -> Option<String> {
        None
    }

    /// Get entity's Tag attribute
    ///
    /// Tag is often used for element identification/marking.
    ///
    /// # Arguments
    /// * `id` - The entity ID
    ///
    /// # Returns
    /// The tag string if available
    fn tag(&self, _id: EntityId) -> Option<String> {
        None
    }
}
