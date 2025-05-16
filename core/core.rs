pub mod command_runner;
pub mod dependency_checker;
pub mod file_finder;
pub mod processor;
pub mod stripper;

pub use command_runner::*;
pub use dependency_checker::check_dependencies;

pub use file_finder::{CliArgs, Command, CompletionArgs, XzenfmtArgs, find_files};

pub use processor::{OperationMode, ProcessedFileResult, process_files};
pub use stripper::{StripError, find_language_comments, remove_matches};

#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File processing error in {path}: {message}")]
    Processing { path: String, message: String },

    #[error("Dependency check failed: {0}")]
    Dependency(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}
