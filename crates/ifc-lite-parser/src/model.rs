// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ParsedModel - Main IFC model implementation

use crate::properties::PropertyReaderImpl;
use crate::resolver::ResolverImpl;
use crate::scanner::{parse_header, EntityScanner};
use crate::spatial::SpatialQueryImpl;
use crate::units::extract_unit_scale;

use ifc_lite_model::{
    EntityId, EntityResolver, IfcModel, IfcType, ModelMetadata, ProgressCallback, PropertyReader,
    Result, SpatialQuery,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Parsed IFC model implementing the `IfcModel` trait
///
/// This is the main entry point for accessing IFC data. It provides access
/// to entities, properties, and spatial structure through trait objects.
pub struct ParsedModel {
    /// Entity resolver for lookups
    resolver: Arc<ResolverImpl>,
    /// Property reader
    properties: Arc<PropertyReaderImpl>,
    /// Spatial query
    spatial: Arc<SpatialQueryImpl>,
    /// Unit scale (file units to meters)
    unit_scale: f64,
    /// File metadata
    metadata: ModelMetadata,
}

impl ParsedModel {
    /// Parse IFC content and create a model
    pub fn parse(content: &str, build_spatial: bool, _extract_properties: bool) -> Result<Self> {
        // Build entity index
        let index = EntityScanner::build_index(content);

        // Build type index during parse
        let mut type_index: FxHashMap<IfcType, Vec<EntityId>> = FxHashMap::default();
        let mut scanner = EntityScanner::new(content);
        while let Some((id, type_name, _, _)) = scanner.next_entity() {
            let ifc_type = IfcType::parse(type_name);
            type_index
                .entry(ifc_type)
                .or_default()
                .push(EntityId(id));
        }

        // Create resolver
        let resolver = Arc::new(ResolverImpl::with_type_index(
            content.to_string(),
            index,
            type_index,
        ));

        // Extract unit scale
        let unit_scale = extract_unit_scale(resolver.as_ref());

        // Create property reader
        let properties = Arc::new(PropertyReaderImpl::new(resolver.clone()));

        // Build spatial structure
        let spatial = if build_spatial {
            Arc::new(SpatialQueryImpl::build(resolver.as_ref()))
        } else {
            Arc::new(SpatialQueryImpl::empty())
        };

        // Parse header metadata
        let header = parse_header(content);
        let metadata = ModelMetadata {
            schema_version: header.schema_version,
            originating_system: header.originating_system,
            preprocessor_version: header.preprocessor_version,
            file_name: header.file_name,
            file_description: None, // TODO: extract from FILE_DESCRIPTION
            author: header.author,
            organization: header.organization,
            timestamp: header.timestamp,
        };

        Ok(Self {
            resolver,
            properties,
            spatial,
            unit_scale,
            metadata,
        })
    }

    /// Parse with progress reporting
    pub fn parse_with_progress(
        content: &str,
        build_spatial: bool,
        _extract_properties: bool,
        on_progress: ProgressCallback,
    ) -> Result<Self> {
        on_progress("Scanning entities", 0.0);

        // Build entity index
        let index = EntityScanner::build_index(content);
        on_progress("Building index", 20.0);

        // Build type index during parse
        let mut type_index: FxHashMap<IfcType, Vec<EntityId>> = FxHashMap::default();
        let mut scanner = EntityScanner::new(content);
        while let Some((id, type_name, _, _)) = scanner.next_entity() {
            let ifc_type = IfcType::parse(type_name);
            type_index
                .entry(ifc_type)
                .or_default()
                .push(EntityId(id));
        }
        on_progress("Indexing types", 40.0);

        // Create resolver
        let resolver = Arc::new(ResolverImpl::with_type_index(
            content.to_string(),
            index,
            type_index,
        ));

        // Extract unit scale
        let unit_scale = extract_unit_scale(resolver.as_ref());
        on_progress("Extracting units", 50.0);

        // Create property reader
        let properties = Arc::new(PropertyReaderImpl::new(resolver.clone()));
        on_progress("Building property index", 60.0);

        // Build spatial structure
        let spatial = if build_spatial {
            on_progress("Building spatial structure", 70.0);
            Arc::new(SpatialQueryImpl::build(resolver.as_ref()))
        } else {
            Arc::new(SpatialQueryImpl::empty())
        };
        on_progress("Processing metadata", 90.0);

        // Parse header metadata
        let header = parse_header(content);
        let metadata = ModelMetadata {
            schema_version: header.schema_version,
            originating_system: header.originating_system,
            preprocessor_version: header.preprocessor_version,
            file_name: header.file_name,
            file_description: None, // TODO: extract from FILE_DESCRIPTION
            author: header.author,
            organization: header.organization,
            timestamp: header.timestamp,
        };

        on_progress("Complete", 100.0);

        Ok(Self {
            resolver,
            properties,
            spatial,
            unit_scale,
            metadata,
        })
    }

    /// Get the resolver (for geometry processing, etc.)
    pub fn resolver_arc(&self) -> Arc<ResolverImpl> {
        self.resolver.clone()
    }
}

impl IfcModel for ParsedModel {
    fn resolver(&self) -> &dyn EntityResolver {
        self.resolver.as_ref()
    }

    fn properties(&self) -> &dyn PropertyReader {
        self.properties.as_ref()
    }

    fn spatial(&self) -> &dyn SpatialQuery {
        self.spatial.as_ref()
    }

    fn unit_scale(&self) -> f64 {
        self.unit_scale
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_IFC: &str = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('test.ifc','2024-01-01T00:00:00',('Author'),('Org'),'Preprocessor','App','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCPROJECT('guid',$,'Test Project',$,$,$,$,$,#2);
#2=IFCUNITASSIGNMENT((#3));
#3=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#4=IFCSITE('guid2',$,'Site',$,$,$,$,$,$,$,$,$,$,$);
#5=IFCRELAGGREGATES('guid3',$,$,$,#1,(#4));
#6=IFCBUILDING('guid4',$,'Building',$,$,$,$,$,$,$,$,$);
#7=IFCRELAGGREGATES('guid5',$,$,$,#4,(#6));
#8=IFCBUILDINGSTOREY('guid6',$,'Ground Floor',$,$,$,$,$,.ELEMENT.,0.0);
#9=IFCRELAGGREGATES('guid7',$,$,$,#6,(#8));
#10=IFCWALL('guid8',$,'Wall 1',$,$,$,$,$);
#11=IFCRELCONTAINEDINSPATIALSTRUCTURE('guid9',$,$,$,(#10),#8);
ENDSEC;
END-ISO-10303-21;
"#;

    #[test]
    fn test_parse_model() {
        let model = ParsedModel::parse(TEST_IFC, true, true).unwrap();

        // Check metadata
        assert_eq!(model.metadata().schema_version, "IFC2X3");
        assert_eq!(model.metadata().file_name, Some("test.ifc".to_string()));

        // Check unit scale (millimeters -> meters = 0.001)
        assert!((model.unit_scale() - 0.001).abs() < 1e-10);

        // Check resolver works
        let walls = model.resolver().find_by_type_name("IFCWALL");
        assert_eq!(walls.len(), 1);

        // Check spatial structure
        let tree = model.spatial().spatial_tree();
        assert!(tree.is_some());
        let project = tree.unwrap();
        assert_eq!(project.name, "Test Project");

        // Check storeys
        let storeys = model.spatial().storeys();
        assert_eq!(storeys.len(), 1);
        assert_eq!(storeys[0].name, "Ground Floor");
    }

    #[test]
    fn test_search() {
        let model = ParsedModel::parse(TEST_IFC, true, true).unwrap();

        // Search by name
        let results = model.spatial().search("wall");
        assert!(!results.is_empty());

        // Search by type
        let results = model.spatial().search("IFCWALL");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_elements_in_storey() {
        let model = ParsedModel::parse(TEST_IFC, true, true).unwrap();

        let storeys = model.spatial().storeys();
        assert!(!storeys.is_empty());

        let elements = model.spatial().elements_in_storey(storeys[0].id);
        assert!(!elements.is_empty());
    }
}
