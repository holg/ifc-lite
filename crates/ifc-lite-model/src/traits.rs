// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Core traits for IFC parsing
//!
//! These traits define the main abstractions for working with IFC data.

use crate::{EntityResolver, GeometrySource, ModelMetadata, PropertyReader, Result, SpatialQuery};
use std::sync::Arc;

/// Progress callback type for parsing operations
pub type ProgressCallback = Box<dyn Fn(&str, f32) + Send>;

/// Main parsing interface - entry point for parsing IFC content
///
/// Implementations of this trait provide the ability to parse IFC file content
/// and return a model that can be queried through various trait interfaces.
///
/// # Example
///
/// ```ignore
/// use ifc_lite_model::{IfcParser, IfcModel};
///
/// let parser: Box<dyn IfcParser> = get_parser();
/// let model = parser.parse(ifc_content)?;
/// println!("Schema: {}", model.metadata().schema_version);
/// ```
pub trait IfcParser: Send + Sync {
    /// Parse IFC content and return a model
    ///
    /// # Arguments
    /// * `content` - The IFC file content as a string
    ///
    /// # Returns
    /// An `Arc<dyn IfcModel>` on success, or a `ParseError` on failure
    fn parse(&self, content: &str) -> Result<Arc<dyn IfcModel>>;

    /// Parse IFC content with progress reporting
    ///
    /// # Arguments
    /// * `content` - The IFC file content as a string
    /// * `on_progress` - Callback function receiving (phase_name, percent_complete)
    ///
    /// # Returns
    /// An `Arc<dyn IfcModel>` on success, or a `ParseError` on failure
    fn parse_with_progress(
        &self,
        content: &str,
        on_progress: ProgressCallback,
    ) -> Result<Arc<dyn IfcModel>>;
}

/// Core model interface - read-only access to a parsed IFC model
///
/// This trait provides access to the various aspects of an IFC model through
/// sub-traits that handle specific concerns (entity resolution, properties,
/// spatial structure, etc.)
///
/// The model is thread-safe (`Send + Sync`) to support parallel processing
/// and use in async contexts.
pub trait IfcModel: Send + Sync {
    /// Get entity resolver for entity lookups and reference resolution
    ///
    /// The resolver provides O(1) access to entities by ID and can resolve
    /// entity references found in attribute values.
    fn resolver(&self) -> &dyn EntityResolver;

    /// Get property reader for accessing property sets and quantities
    ///
    /// The property reader provides access to IfcPropertySet and IfcElementQuantity
    /// data associated with entities.
    fn properties(&self) -> &dyn PropertyReader;

    /// Get spatial query interface for hierarchy traversal
    ///
    /// The spatial query interface provides access to the spatial structure
    /// (Project → Site → Building → Storey → Elements) and search capabilities.
    fn spatial(&self) -> &dyn SpatialQuery;

    /// Get unit scale factor (file units to meters)
    ///
    /// This value should be used to convert coordinates from the file's
    /// native unit system to meters. Common values:
    /// - 1.0 for meters
    /// - 0.001 for millimeters
    /// - 0.0254 for inches
    /// - 0.3048 for feet
    fn unit_scale(&self) -> f64;

    /// Get file metadata (schema version, originating system, etc.)
    fn metadata(&self) -> &ModelMetadata;
}

/// Extension trait for models that support geometry processing
///
/// This trait is separate from `IfcModel` to allow for models that only
/// provide metadata/property access without geometry processing capability.
pub trait GeometryModel: IfcModel {
    /// Get geometry source for rendering
    ///
    /// The geometry source provides processed mesh data ready for GPU rendering.
    fn geometry(&self) -> &dyn GeometrySource;
}

/// Trait for models that can be extended dynamically
///
/// This allows adding capabilities to a model after initial parsing.
pub trait ExtensibleModel: IfcModel {
    /// Check if geometry processing is available
    fn has_geometry(&self) -> bool;

    /// Get geometry source if available
    fn try_geometry(&self) -> Option<&dyn GeometrySource>;
}
