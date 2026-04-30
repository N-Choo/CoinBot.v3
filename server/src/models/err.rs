use std::env::VarError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    // 2. Define how the error is displayed to the user
    // 3. Use #[from] to automatically generate the `impl From<sqlx::Error>` block!
    #[error("Internal database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("Node with ID {0} already exists")]
    NodeExists(i32),

    #[error("Invalid input provided: {0}")]
    Input(String),

    #[error("Missing environment variable: {0}")]
    Env404(#[from] VarError), // Generates `impl From<VarError>` automatically

    #[error("Resource not found: {0}")]
    NotFound(String),
}
