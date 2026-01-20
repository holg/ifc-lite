// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! IFC-Lite Parser - High-performance IFC parser
//!
//! This crate provides a fast, memory-efficient parser for IFC (STEP) files.
//! It implements the traits defined in `ifc-lite-model` for a clean abstraction.
//!
//! # Features
//!
//! - **Fast tokenization** using `nom` combinators
//! - **SIMD-accelerated scanning** using `memchr`
//! - **Lazy entity decoding** - only parse entities when needed
//! - **Arc-based caching** - efficient memory sharing
//! - **Progress reporting** for large files
//!
//! # Example
//!
//! ```ignore
//! use ifc_lite_parser::StepParser;
//! use ifc_lite_model::IfcParser;
//!
//! let parser = StepParser::new();
//! let model = parser.parse(ifc_content)?;
//!
//! // Access entities
//! let walls = model.resolver().find_by_type_name("IFCWALL");
//! println!("Found {} walls", walls.len());
//! ```

mod decoder;
mod model;
mod properties;
mod resolver;
mod scanner;
mod spatial;
mod tokenizer;
mod units;

pub use decoder::EntityDecoder;
pub use model::ParsedModel;
pub use scanner::EntityScanner;
pub use tokenizer::{parse_entity, Token};

use ifc_lite_model::{IfcModel, IfcParser, ProgressCallback, Result};
use std::sync::Arc;

/// Main STEP/IFC parser implementing `IfcParser` trait
///
/// This is the entry point for parsing IFC files. It creates a `ParsedModel`
/// that provides access to all IFC data through the trait interfaces.
#[derive(Default)]
pub struct StepParser {
    /// Whether to build spatial tree during parsing
    pub build_spatial_tree: bool,
    /// Whether to extract properties during parsing
    pub extract_properties: bool,
}

impl StepParser {
    /// Create a new parser with default settings
    pub fn new() -> Self {
        Self {
            build_spatial_tree: true,
            extract_properties: true,
        }
    }

    /// Create a parser optimized for geometry-only access
    pub fn geometry_only() -> Self {
        Self {
            build_spatial_tree: false,
            extract_properties: false,
        }
    }

    /// Set whether to build spatial tree
    pub fn with_spatial_tree(mut self, enabled: bool) -> Self {
        self.build_spatial_tree = enabled;
        self
    }

    /// Set whether to extract properties
    pub fn with_properties(mut self, enabled: bool) -> Self {
        self.extract_properties = enabled;
        self
    }
}

impl IfcParser for StepParser {
    fn parse(&self, content: &str) -> Result<Arc<dyn IfcModel>> {
        ParsedModel::parse(content, self.build_spatial_tree, self.extract_properties)
            .map(|m| Arc::new(m) as Arc<dyn IfcModel>)
    }

    fn parse_with_progress(
        &self,
        content: &str,
        on_progress: ProgressCallback,
    ) -> Result<Arc<dyn IfcModel>> {
        ParsedModel::parse_with_progress(
            content,
            self.build_spatial_tree,
            self.extract_properties,
            on_progress,
        )
        .map(|m| Arc::new(m) as Arc<dyn IfcModel>)
    }
}

/// Quick parse function for simple use cases
pub fn parse(content: &str) -> Result<Arc<dyn IfcModel>> {
    StepParser::new().parse(content)
}

/// Parse with progress reporting
pub fn parse_with_progress(
    content: &str,
    on_progress: impl Fn(&str, f32) + Send + 'static,
) -> Result<Arc<dyn IfcModel>> {
    StepParser::new().parse_with_progress(content, Box::new(on_progress))
}
