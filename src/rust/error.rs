//! Error types for the Gnomics framework.
//!
//! This module provides a unified error type for all operations in the Gnomics
//! framework, using the `thiserror` crate for ergonomic error handling.

use thiserror::Error;

/// The main error type for Gnomics operations.
///
/// This enum represents all possible error conditions that can occur
/// during the execution of Gnomics operations.
#[derive(Error, Debug)]
pub enum GnomicsError {
    /// Block has not been initialized before use
    #[error("Block not initialized - call init() before use")]
    NotInitialized,

    /// Input size does not match expected size
    #[error("Invalid input size: expected {expected}, got {actual}")]
    InvalidInputSize {
        /// Expected size
        expected: usize,
        /// Actual size received
        actual: usize,
    },

    /// Invalid parameter value
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Index out of bounds
    #[error("Index out of bounds: index {index}, length {length}")]
    IndexOutOfBounds {
        /// The index that was accessed
        index: usize,
        /// The valid length
        length: usize,
    },

    /// I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error occurred
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// Generic error with custom message
    #[error("{0}")]
    Other(String),
}

/// A specialized `Result` type for Gnomics operations.
///
/// This is a type alias for `Result<T, GnomicsError>` and is used
/// throughout the Gnomics codebase for consistency.
pub type Result<T> = std::result::Result<T, GnomicsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = GnomicsError::NotInitialized;
        assert_eq!(
            err.to_string(),
            "Block not initialized - call init() before use"
        );

        let err = GnomicsError::InvalidInputSize {
            expected: 1024,
            actual: 512,
        };
        assert_eq!(
            err.to_string(),
            "Invalid input size: expected 1024, got 512"
        );
    }

    #[test]
    fn test_result_type() {
        fn returns_result() -> Result<i32> {
            Ok(42)
        }

        assert_eq!(returns_result().unwrap(), 42);
    }
}
