use std::ffi::CStr;
use std::fmt::{self, Display};

use sdl3::sys::error::SDL_GetError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    call: &'static str,
    message: String,
}

impl Error {
    pub fn new(call: &'static str) -> Self {
        Self {
            call,
            message: last_message(),
        }
    }

    pub fn with_message(call: &'static str, message: impl Into<String>) -> Self {
        Self {
            call,
            message: message.into(),
        }
    }

    #[inline]
    pub fn call(&self) -> &'static str {
        self.call
    }

    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Not every failure sets a message.
        if self.message.is_empty() {
            write!(f, "{} failed", self.call)
        } else {
            write!(f, "{} failed: {}", self.call, self.message)
        }
    }
}

impl std::error::Error for Error {}

pub fn last_message() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()) }
        .to_string_lossy()
        .into_owned()
}

pub(crate) fn check(call: &'static str, ok: bool) -> Result<()> {
    if ok { Ok(()) } else { Err(Error::new(call)) }
}

/// NULL is how SDL's pointer entry points report failure.
pub(crate) fn check_ptr<T>(call: &'static str, ptr: *mut T) -> Result<*mut T> {
    if ptr.is_null() {
        Err(Error::new(call))
    } else {
        Ok(ptr)
    }
}
