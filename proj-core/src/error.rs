use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unknown CRS: {0}")]
    UnknownCrs(String),

    #[error("unsupported projection: {0}")]
    UnsupportedProjection(String),

    #[error("unknown operation: {0}")]
    UnknownOperation(String),

    #[error("operation selection failed: {0}")]
    OperationSelection(String),

    #[error("invalid CRS definition: {0}")]
    InvalidDefinition(String),

    #[error("coordinate out of range: {0}")]
    OutOfRange(String),

    #[error(transparent)]
    Grid(#[from] crate::grid::GridError),
}
