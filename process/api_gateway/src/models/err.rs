use std::env::VarError;
use thiserror::Error;

/// Application-level errors shared across handlers.
#[derive(Debug, Error)]
pub enum AppError {
    // 2. Define how the error is displayed to the user
    // 3. Use #[from] to automatically generate the `impl From<sqlx::Error>` block!
    /// A database operation failed.
    #[error("Internal database error: {0}")]
    Db(#[from] sqlx::Error),

    /// A node with the given id already exists.
    #[error("Node with ID {0} already exists")]
    NodeExists(i32),

    /// The caller supplied invalid input.
    #[error("Invalid input provided: {0}")]
    Input(String),

    /// A required environment variable is missing.
    #[error("Missing environment variable: {0}")]
    Env404(#[from] VarError), // Generates `impl From<VarError>` automatically

    /// The requested resource does not exist.
    #[error("Resource not found: {0}")]
    NotFound(String),
}
