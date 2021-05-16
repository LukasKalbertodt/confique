use std::{ffi::OsString, fmt, path::PathBuf};

use serde::Deserialize;

#[doc(hidden)]
pub mod internal;

mod file;


pub use serde;
pub use confique_macro::Config;
pub use self::file::{File, FileFormat};


/// A configuration object that can be deserialized in layers via `serde`.
///
/// You would usually derive this trait for your own type and then load the
/// configuration with one of the provided methods, like
/// [`from_sources`][Self::from_sources].
///
/// # Deriving
///
/// This trait is usually derived as implementing it manually usually entails
/// writing some boilerplate code, that goes against the "don't repeat yourself"
/// principle.
///
/// TODO
pub trait Config: Sized {
    /// A version of `Self` that represents a potetially partial configuration.
    ///
    /// This type is supposed to have the exact same fields as this one, but
    /// with every field being optional. Its main use is to have a layered
    /// configuration from multiple sources where each layer might not contain
    /// all required values. The only thing that matters is that combining all
    /// layers will result in a configuration object that has all required
    /// values defined.
    type Partial: Partial;

    /// Tries to create `Self` from a potentially partial object.
    ///
    /// If any required values are not defined in `partial`, an [`Error`] is
    /// returned.
    fn from_partial(partial: Self::Partial) -> Result<Self, Error>;

    /// Tries to load configuration values from all given sources, merging all
    /// layers and returning the result. Sources earlier in the given slice have
    /// a higher priority.
    ///
    /// TODO: example
    fn from_sources(sources: &[&dyn Source<Self>]) -> Result<Self, Error> {
        let mut partial = Self::Partial::default_values();
        for src in sources.iter().rev() {
            let layer = src.load()?;
            partial = layer.with_fallback(partial);
        }

        Self::from_partial(partial)
    }
}

/// A potentially partial configuration object that can be directly deserialized
/// via `serde`.
pub trait Partial: for<'de> Deserialize<'de> {
    /// Returns `Self` where all fields/values are `None` or empty.
    fn empty() -> Self;

    /// Returns an object containing all default values (i.e. set via
    /// `#[config(default = ...)]` when deriving `Config`) with all remaining
    /// values/fields set to `None`/being empty.
    fn default_values() -> Self;

    /// Combines two partial configuration objects. `self` has a higher
    /// priority; missing values in `self` are filled with values in `fallback`,
    /// if they exist. The semantics of this method is basically like in
    /// [`Option::or`].
    fn with_fallback(self, fallback: Self) -> Self;
}

/// A source of configuration values for the configuration object `T`, e.g. a
/// file or environment variables.
pub trait Source<C: Config> {
    /// Attempts to load a potentially partially configuration object.
    fn load(&self) -> Result<C::Partial, Error>;
}

/// Type describing all errors that can occur in this library.
pub struct Error {
    inner: Box<ErrorInner>,
}

enum ErrorInner {
    /// Returned by `Config::from_partial` when the partial does not contain
    /// values for all required configuration values. The string is a
    /// human-readable path to the value, e.g. `http.port`.
    MissingValue(String),

    /// An IO error occured, e.g. when reading a file.
    Io {
        path: Option<PathBuf>,
        err: std::io::Error,
    },

    /// Returned by `Source::load` implementations when deserialization fails.
    Deserialization {
        /// A human readable description for the error message, describing from
        /// what source it was attempted to deserialize. Completes the sentence
        /// "failed to deserialize configuration from ". E.g. "file 'foo.toml'"
        /// or "environment variable 'FOO_PORT'".
        source: Option<String>,
        err: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Returned by the [`Source`] impls for `Path` and `PathBuf` if the file
    /// extension is not supported by confique or if the corresponding Cargo
    /// feature of confique was not enabled.
    UnsupportedFileFormat {
        path: PathBuf,
        extension: OsString,
    },

    /// Returned by the [`Source`] impls for `Path` and `PathBuf` if the path
    /// does not contain a file extension.
    MissingFileExtension {
        path: PathBuf,
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &*self.inner {
            ErrorInner::Io { err, .. } => Some(err),
            ErrorInner::Deserialization { err, .. } => Some(&**err),
            ErrorInner::MissingValue(_)
            | ErrorInner::UnsupportedFileFormat { .. }
            | ErrorInner::MissingFileExtension { .. } => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.inner {
            ErrorInner::MissingValue(path) => {
                std::write!(f, "required configuration value is missing: '{}'", path)
            }
            ErrorInner::Io { path: Some(path), .. } => {
                std::write!(f,
                    "IO error occured while reading configuration file '{}'",
                    path.display(),
                )
            }
            ErrorInner::Io { path: None, .. } => {
                std::write!(f, "IO error occured while loading configuration")
            }
            ErrorInner::Deserialization { source: Some(source), .. } => {
                std::write!(f, "failed to deserialize configuration from {}", source)
            }
            ErrorInner::Deserialization { source: None, .. } => {
                std::write!(f, "failed to deserialize configuration")
            }
            ErrorInner::UnsupportedFileFormat { path, extension } => {
                std::write!(f,
                    "unknown configuration file format '{}' of '{}'",
                    extension.to_string_lossy(),
                    path.display(),
                )
            }
            ErrorInner::MissingFileExtension { path } => {
                std::write!(f,
                    "cannot guess configuration file format due to missing file extension in '{}'",
                    path.display(),
                )
            }
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<ErrorInner> for Error {
    fn from(inner: ErrorInner) -> Self {
        Self { inner: Box::new(inner) }
    }
}
