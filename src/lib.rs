use std::fmt;

use serde::Deserialize;


pub use serde;
pub use confique_macro::Config;


// #[cfg(feature = "doc-example")]
// pub mod example;

#[doc(hidden)]
pub mod internal;


pub trait Config: Sized {
    type Partial: Partial;

    fn from_partial(partial: Self::Partial) -> Result<Self, Error>;
}

pub trait Partial: for<'de> Deserialize<'de> {
    fn empty() -> Self;
    fn default_values() -> Self;
    fn with_fallback(self, fallback: Self) -> Self;
}


pub enum Error {
    /// Returned by `Config::from_partial` when the partial does not contain
    /// values for all required configuration values. The string is a
    /// human-readable path to the value, e.g. `http.port`.
    MissingValue(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::MissingValue(path) => {
                std::write!(f, "required configuration value is missing: '{}'", path)
            }
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
