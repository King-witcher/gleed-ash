//! No C++ equivalent: there `vulkan_raii.hpp` threw `vk::SystemError` on its
//! own for every failing call, nobody caught it, and the error climbed until
//! the process died. The bindings are 1:1 with the C API and throw nothing —
//! every call returns `VkResult<T>`, so the choice becomes explicit.
//!
//! The whole engine uses the same rule on both sides:
//!
//! - **Environment error** — no compatible GPU, SDL could not open the window,
//!   the driver ran out of memory, the device was lost. Becomes an [`Error`]
//!   and rises with `?` up to `main`, which prints the chain and exits with an
//!   error code.
//! - **Our bug** — mapping a buffer allocated in pure VRAM, dropping a
//!   `Frame` without submitting, embedding invalid SPIR-V in the binary.
//!   Stays `panic!`/`expect`: there is nothing the caller can do, and a stack
//!   trace at the point of failure is worth more than a `Result`.
//!
//! [`Context`] exists so the information the original
//! `expect("Failed to create command pool")` carried is not thrown away:
//! `.context("create the command pool")?` propagates the error AND says which call
//! failed.

use std::fmt;

use gpu_allocator::AllocationError;

/// Same pattern as `io::Result`: the alias hides the error type, which is the
/// same across the whole crate.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// A Vulkan call returned an error `VkResult`.
    Vulkan {
        op: &'static str,
        result: vk::Result,
    },
    /// `gpu-allocator` could not sub-allocate `VkDeviceMemory`.
    Allocation {
        op: &'static str,
        source: AllocationError,
    },
    /// SDL failed: init, video subsystem, window, event pump or surface.
    Sdl { op: &'static str, message: String },
    /// The machine has Vulkan, but does not meet one of this engine's
    /// requirements.
    Unsupported(String),
}

impl Error {
    pub fn unsupported(what: impl Into<String>) -> Self {
        Error::Unsupported(what.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Vulkan { op, result } => write!(f, "failed to {op}: {result}"),
            Error::Allocation { op, source } => write!(f, "failed to {op}: {source}"),
            Error::Sdl { op, message } => write!(f, "failed to {op}: {message}"),
            Error::Unsupported(what) => write!(f, "unsupported: {what}"),
        }
    }
}

impl std::error::Error for Error {
    /// Lets `main` walk the chain (`source()`) and print the original cause
    /// along with each level's context.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Vulkan { result, .. } => Some(result),
            Error::Allocation { source, .. } => Some(source),
            Error::Sdl { .. } | Error::Unsupported(_) => None,
        }
    }
}

/// Converts a library error into [`Error`], attaching the operation name.
///
/// This is what an `impl From<E> for Error` could not do: `From` would have to
/// pick one fixed text per error type, while here the text comes from the
/// exact call site. In exchange, `?` alone is not enough — it is always
/// `.context(..)?`.
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

// SDL has two error types (the generic one and the window builder's), both
// carrying only text, so they land in the same variant.
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

/// Replaces `.expect("Failed to create command pool")` with
/// `.context("create the command pool")?`: same message, except the error rises
/// instead of killing the process on the spot.
pub trait Context<T> {
    fn context(self, op: &'static str) -> Result<T>;
}

impl<T, E: IntoError> Context<T> for std::result::Result<T, E> {
    fn context(self, op: &'static str) -> Result<T> {
        self.map_err(|error| error.into_error(op))
    }
}
