
pub use confique_macro::config as config;

pub use serde;


use std::fmt;


/// Error for the `TryFrom` conversion from raw types to the main types.
///
/// This error is returned when a required value is `None` in the raw type.
#[derive(Clone)]
pub struct TryFromError {
    /// This is only public so that macro generated code can created instances
    /// of this type.
    #[doc(hidden)]
    pub path: &'static str,
}

impl fmt::Display for TryFromError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::write!(f, "required configuration value is missing: '{}'", self.path)
    }
}

impl fmt::Debug for TryFromError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for TryFromError {}
