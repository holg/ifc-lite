// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Error types for geometry processing

use thiserror::Error;

/// Geometry processing result type
pub type Result<T> = std::result::Result<T, Error>;

/// Geometry processing errors
#[derive(Error, Debug)]
pub enum Error {
    /// Geometry processing error
    #[error("Geometry error: {0}")]
    Geometry(String),

    /// Missing entity error
    #[error("Entity not found: #{0}")]
    EntityNotFound(u32),

    /// Invalid attribute error
    #[error("Invalid attribute at index {index}: {message}")]
    InvalidAttribute { index: usize, message: String },

    /// Profile processing error
    #[error("Profile error: {0}")]
    Profile(String),

    /// Triangulation error
    #[error("Triangulation error: {0}")]
    Triangulation(String),

    /// CSG operation error
    #[error("CSG error: {0}")]
    Csg(String),

    /// Unsupported geometry type
    #[error("Unsupported geometry type: {0}")]
    UnsupportedType(String),
}

impl Error {
    /// Create a geometry error
    pub fn geometry(msg: impl Into<String>) -> Self {
        Error::Geometry(msg.into())
    }

    /// Create a profile error
    pub fn profile(msg: impl Into<String>) -> Self {
        Error::Profile(msg.into())
    }

    /// Create a triangulation error
    pub fn triangulation(msg: impl Into<String>) -> Self {
        Error::Triangulation(msg.into())
    }

    /// Create a CSG error
    pub fn csg(msg: impl Into<String>) -> Self {
        Error::Csg(msg.into())
    }

    /// Create an entity not found error
    pub fn entity_not_found(id: u32) -> Self {
        Error::EntityNotFound(id)
    }

    /// Create an invalid attribute error
    pub fn invalid_attribute(index: usize, msg: impl Into<String>) -> Self {
        Error::InvalidAttribute {
            index,
            message: msg.into(),
        }
    }

    /// Create an unsupported type error
    pub fn unsupported_type(type_name: impl Into<String>) -> Self {
        Error::UnsupportedType(type_name.into())
    }
}
