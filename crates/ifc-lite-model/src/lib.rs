// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! IFC-Lite Model - Trait definitions and shared types for IFC parsing
//!
//! This crate provides the core abstractions for working with IFC (Industry Foundation Classes)
//! files. It defines traits that can be implemented by different parser backends, allowing
//! consumers to work with IFC data in a backend-agnostic way.
//!
//! # Architecture
//!
//! The crate is organized around several key traits:
//!
//! - [`IfcParser`] - Entry point for parsing IFC content
//! - [`IfcModel`] - Read-only access to a parsed IFC model
//! - [`EntityResolver`] - Entity lookup and reference resolution
//! - [`PropertyReader`] - Access to property sets and quantities
//! - [`SpatialQuery`] - Spatial hierarchy traversal and search
//! - [`GeometrySource`] - Geometry data for rendering (optional extension)
//!
//! # Example
//!
//! ```ignore
//! use ifc_lite_model::{IfcParser, IfcModel, EntityId};
//!
//! // Use any parser that implements IfcParser
//! let parser: Box<dyn IfcParser> = get_parser();
//! let model = parser.parse(ifc_content)?;
//!
//! // Access data through trait interfaces
//! let resolver = model.resolver();
//! if let Some(entity) = resolver.get(EntityId(123)) {
//!     println!("Entity type: {:?}", entity.ifc_type);
//! }
//! ```

pub mod error;
pub mod geometry;
pub mod properties;
pub mod resolver;
pub mod spatial;
pub mod traits;
pub mod types;

// Re-export all public types
pub use error::*;
pub use geometry::*;
pub use properties::*;
pub use resolver::*;
pub use spatial::*;
pub use traits::*;
pub use types::*;
