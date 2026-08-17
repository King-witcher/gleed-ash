use gpu_allocator::AllocationError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A Vulkan call returned an error `VkResult`.
    #[error("failed to {op}: {result}")]
    Vulkan {
        op: &'static str,
        #[source]
        result: vk::Result,
    },
    /// `gpu-allocator` could not sub-allocate `VkDeviceMemory`.
    #[error("failed to {op}: {source}")]
    Allocation {
        op: &'static str,
        source: AllocationError,
    },
    /// SDL failed: init, video subsystem, window, event pump or surface.
    #[error("failed to {op}: {message}")]
    Sdl { op: &'static str, message: String },
    /// The machine has Vulkan, but does not meet one of this engine's
    /// requirements.
    #[error("unsupported: {0}")]
    Unsupported(String),
}

impl Error {
    pub fn unsupported(what: impl Into<String>) -> Self {
        Error::Unsupported(what.into())
    }
}

pub trait IntoError {
    fn into_error(self, op: &'static str) -> Error;
}

impl IntoError for vk::Result {
    fn into_error(self, op: &'static str) -> Error {
        Error::Vulkan { op, result: self }
    }
}

impl IntoError for AllocationError {
    fn into_error(self, op: &'static str) -> Error {
        Error::Allocation { op, source: self }
    }
}

impl IntoError for sdl3::Error {
    fn into_error(self, op: &'static str) -> Error {
        Error::Sdl {
            op,
            message: self.to_string(),
        }
    }
}

impl IntoError for sdl3::video::WindowBuildError {
    fn into_error(self, op: &'static str) -> Error {
        Error::Sdl {
            op,
            message: self.to_string(),
        }
    }
}

pub trait Context<T> {
    fn context(self, op: &'static str) -> Result<T>;
}

impl<T, E: IntoError> Context<T> for std::result::Result<T, E> {
    fn context(self, op: &'static str) -> Result<T> {
        self.map_err(|error| error.into_error(op))
    }
}
