// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! IFC5 (IFCX) JSON format parser
//!
//! This module handles parsing of IFC5 files which use a JSON-based format
//! with Entity Component System (ECS) composition model.
//!
//! Key differences from IFC4 (STEP):
//! - JSON format instead of STEP text
//! - Path-based UUIDs instead of Express IDs (#1, #2, ...)
//! - Pre-tessellated USD geometry instead of parametric
//! - Flat namespaced attributes instead of explicit relationship entities
//! - ECS composition via `children` and `inherits`

mod composition;
mod geometry;
mod model;
mod types;

pub use composition::compose_nodes;
pub use geometry::IfcxGeometry;
pub use model::IfcxModel;
pub use types::*;

use crate::Result;

/// Detect if content is IFCX (JSON) format
///
/// IFC5 files start with '{' (JSON object) while IFC4 files start with 'ISO-10303-21'
#[inline]
pub fn is_ifcx_format(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{')
}

/// Parse IFCX content into a model
pub fn parse_ifcx(content: &str) -> Result<IfcxModel> {
    IfcxModel::parse(content)
}
