pub mod budget;
pub mod redundancy;
pub mod extractive;

pub use budget::BudgetAllocator;
pub use redundancy::RedundancyRemover;
pub use extractive::ExtractiveCompressor;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("budget too small: need at least {minimum} tokens, got {actual}")]
    BudgetTooSmall { minimum: usize, actual: usize },
    #[error("no segments to compress")]
    NoSegments,
    #[error("compression failed: {0}")]
    Failed(String),
}
