// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Unit scale extraction from IFC files

use ifc_lite_model::{AttributeValue, DecodedEntity, EntityResolver, IfcType};

/// Extract unit scale from IFC model
///
/// Finds IFCPROJECT and extracts the length unit conversion factor.
/// Returns 1.0 if no unit information is found.
pub fn extract_unit_scale(resolver: &dyn EntityResolver) -> f64 {
    // Find IFCPROJECT
    let projects = resolver.entities_by_type(&IfcType::IfcProject);
    if projects.is_empty() {
        return 1.0;
    }

    let project = &projects[0];

    // IFCPROJECT has UnitsInContext at index 8
    let units_ref = match project.get(8) {
        Some(AttributeValue::EntityRef(id)) => *id,
        _ => return 1.0,
    };

    // Get IFCUNITASSIGNMENT
    let unit_assignment = match resolver.get(units_ref) {
        Some(entity) => entity,
        None => return 1.0,
    };

    // IFCUNITASSIGNMENT has Units list at index 0
    let units_list = match unit_assignment.get(0) {
        Some(AttributeValue::List(list)) => list,
        _ => return 1.0,
    };

    // Find the length unit
    for unit_attr in units_list {
        if let AttributeValue::EntityRef(unit_id) = unit_attr {
            if let Some(unit) = resolver.get(*unit_id) {
                if let Some(scale) = extract_length_unit_scale(&unit, resolver) {
                    return scale;
                }
            }
        }
    }

    1.0
}

/// Extract scale from a unit entity (IFCSIUNIT or IFCCONVERSIONBASEDUNIT)
fn extract_length_unit_scale(unit: &DecodedEntity, resolver: &dyn EntityResolver) -> Option<f64> {
    match unit.ifc_type {
        IfcType::IfcSIUnit => extract_si_unit_scale(unit),
        IfcType::IfcConversionBasedUnit => extract_conversion_unit_scale(unit, resolver),
        _ => None,
    }
}

/// Extract scale from IFCSIUNIT
///
/// IFCSIUNIT(*, UnitType, Prefix, Name)
/// - UnitType: .LENGTHUNIT., .AREAUNIT., etc.
/// - Prefix: .MILLI., .CENTI., .KILO., etc. or $
/// - Name: .METRE., .SQUARE_METRE., etc.
fn extract_si_unit_scale(unit: &DecodedEntity) -> Option<f64> {
    // Check if this is a length unit (attribute 1)
    let unit_type = unit.get_enum(1)?;
    if unit_type != "LENGTHUNIT" {
        return None;
    }

    // Get prefix (attribute 2)
    let prefix = unit.get(2);
    let prefix_scale = match prefix {
        Some(AttributeValue::Enum(p)) => match p.as_str() {
            "EXA" => 1e18,
            "PETA" => 1e15,
            "TERA" => 1e12,
            "GIGA" => 1e9,
            "MEGA" => 1e6,
            "KILO" => 1e3,
            "HECTO" => 1e2,
            "DECA" => 1e1,
            "DECI" => 1e-1,
            "CENTI" => 1e-2,
            "MILLI" => 1e-3,
            "MICRO" => 1e-6,
            "NANO" => 1e-9,
            "PICO" => 1e-12,
            "FEMTO" => 1e-15,
            "ATTO" => 1e-18,
            _ => 1.0,
        },
        Some(AttributeValue::Null) | Some(AttributeValue::Derived) | None => 1.0,
        _ => 1.0,
    };

    // Get base unit name (attribute 3)
    let name = unit.get_enum(3)?;

    let base_scale = match name {
        "METRE" => 1.0,
        _ => return None, // Not a length unit
    };

    Some(prefix_scale * base_scale)
}

/// Extract scale from IFCCONVERSIONBASEDUNIT
///
/// IFCCONVERSIONBASEDUNIT(Dimensions, UnitType, Name, ConversionFactor)
fn extract_conversion_unit_scale(
    unit: &DecodedEntity,
    resolver: &dyn EntityResolver,
) -> Option<f64> {
    // Check if this is a length unit (attribute 1)
    let unit_type = unit.get_enum(1)?;
    if unit_type != "LENGTHUNIT" {
        return None;
    }

    // Get conversion factor (attribute 3)
    let factor_ref = unit.get_ref(3)?;
    let factor_entity = resolver.get(factor_ref)?;

    // IFCMEASUREWITHUNIT(ValueComponent, UnitComponent)
    if factor_entity.ifc_type != IfcType::IfcMeasureWithUnit {
        return None;
    }

    // Get value (attribute 0)
    let value = extract_measure_value(factor_entity.get(0)?)?;

    // Get unit component for recursive scale
    let unit_ref = factor_entity.get_ref(1)?;
    let base_unit = resolver.get(unit_ref)?;

    let base_scale = extract_length_unit_scale(&base_unit, resolver).unwrap_or(1.0);

    Some(value * base_scale)
}

/// Extract numeric value from a measure attribute
fn extract_measure_value(attr: &AttributeValue) -> Option<f64> {
    match attr {
        AttributeValue::Float(f) => Some(*f),
        AttributeValue::Integer(i) => Some(*i as f64),
        AttributeValue::TypedValue(_, args) if !args.is_empty() => extract_measure_value(&args[0]),
        _ => None,
    }
}

/// Common unit scales for reference
pub mod scales {
    /// Meters to meters (identity)
    pub const METRE: f64 = 1.0;
    /// Millimeters to meters
    pub const MILLIMETRE: f64 = 0.001;
    /// Centimeters to meters
    pub const CENTIMETRE: f64 = 0.01;
    /// Kilometers to meters
    pub const KILOMETRE: f64 = 1000.0;
    /// Inches to meters
    pub const INCH: f64 = 0.0254;
    /// Feet to meters
    pub const FOOT: f64 = 0.3048;
    /// Yards to meters
    pub const YARD: f64 = 0.9144;
    /// Miles to meters
    pub const MILE: f64 = 1609.344;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_scales() {
        // Just verify the scales module is accessible
        assert!((scales::MILLIMETRE - 0.001).abs() < 1e-10);
        assert!((scales::INCH - 0.0254).abs() < 1e-10);
        assert!((scales::FOOT - 0.3048).abs() < 1e-10);
    }
}
