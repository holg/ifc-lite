// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Error types for IFC parsing operations

use crate::EntityId;
use thiserror::Error;

/// Result type alias for parser operations
pub type Result<T> = std::result::Result<T, ParseError>;

/// Errors that can occur during IFC parsing
#[derive(Error, Debug)]
pub enum ParseError {
    /// Invalid IFC file format
    #[error("Invalid IFC format: {0}")]
    InvalidFormat(String),

    /// Failed to parse header section
    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    /// Failed to parse entity
    #[error("Failed to parse entity {0}: {1}")]
    EntityParse(EntityId, String),

    /// Entity not found
    #[error("Entity {0} not found")]
    EntityNotFound(EntityId),

    /// Invalid entity reference
    #[error("Invalid entity reference at {entity}: attribute {attribute}")]
    InvalidReference { entity: EntityId, attribute: usize },

    /// Type mismatch when accessing attribute
    #[error(
        "Type mismatch at entity {entity} attribute {attribute}: expected {expected}, got {actual}"
    )]
    TypeMismatch {
        entity: EntityId,
        attribute: usize,
        expected: String,
        actual: String,
    },

    /// Missing required attribute
    #[error("Missing required attribute {attribute} on entity {entity}")]
    MissingAttribute { entity: EntityId, attribute: usize },

    /// Unsupported IFC schema version
    #[error("Unsupported schema version: {0}")]
    UnsupportedSchema(String),

    /// Geometry processing error
    #[error("Geometry error for entity {entity}: {message}")]
    Geometry { entity: EntityId, message: String },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

impl ParseError {
    /// Create a new format error
    pub fn format(msg: impl Into<String>) -> Self {
        ParseError::InvalidFormat(msg.into())
    }

    /// Create a new entity parse error
    pub fn entity_parse(id: EntityId, msg: impl Into<String>) -> Self {
        ParseError::EntityParse(id, msg.into())
    }

    /// Create a new geometry error
    pub fn geometry(entity: EntityId, msg: impl Into<String>) -> Self {
        ParseError::Geometry {
            entity,
            message: msg.into(),
        }
    }

    /// Create a generic error
    pub fn other(msg: impl Into<String>) -> Self {
        ParseError::Other(msg.into())
    }
}
