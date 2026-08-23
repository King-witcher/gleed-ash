#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to files")]
    FsError(#[from] std::io::Error),

    #[error("invalid image format")]
    InvalidImage(#[from] image::error::ImageError),

    #[error("resource not found")]
    ResourceNotFound,

    #[error("resource extension not supported")]
    UnsupportedExtension,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
