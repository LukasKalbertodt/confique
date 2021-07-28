use std::fmt;

#[cfg(any(feature = "toml", feature = "yaml"))]
use std::path::PathBuf;



/// Type describing all errors that can occur in this library.
pub struct Error {
    pub(crate) inner: Box<ErrorInner>,
}

pub(crate) enum ErrorInner {
    /// Returned by `Config::from_partial` when the partial does not contain
    /// values for all required configuration values. The string is a
    /// human-readable path to the value, e.g. `http.port`.
    MissingValue(String),

    /// An IO error occured, e.g. when reading a file.
    #[cfg(any(feature = "toml", feature = "yaml"))]
    Io {
        path: Option<PathBuf>,
        err: std::io::Error,
    },

    /// Returned by `Source::load` implementations when deserialization fails.
    #[cfg(any(feature = "toml", feature = "yaml"))]
    Deserialization {
        /// A human readable description for the error message, describing from
        /// what source it was attempted to deserialize. Completes the sentence
        /// "failed to deserialize configuration from ". E.g. "file 'foo.toml'"
        /// or "environment variable 'FOO_PORT'".
        source: Option<String>,
        err: Box<dyn std::error::Error + Send + Sync>,
    },

    /// When deserialization via `env` fails. The string is what is passed to
    /// `serde::de::Error::custom`.
    EnvDeserialization {
        field: String,
        key: String,
        msg: String,
    },

    /// Returned by the [`Source`] impls for `Path` and `PathBuf` if the file
    /// extension is not supported by confique or if the corresponding Cargo
    /// feature of confique was not enabled.
    #[cfg(any(feature = "toml", feature = "yaml"))]
    UnsupportedFileFormat {
        path: PathBuf,
    },

    /// Returned by the [`Source`] impls for `Path` and `PathBuf` if the path
    /// does not contain a file extension.
    #[cfg(any(feature = "toml", feature = "yaml"))]
    MissingFileExtension {
        path: PathBuf,
    },

    /// A file source was marked as required but the file does not exist.
    #[cfg(any(feature = "toml", feature = "yaml"))]
    MissingRequiredFile {
        path: PathBuf,
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &*self.inner {
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::Io { err, .. } => Some(err),
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::Deserialization { err, .. } => Some(&**err),
            ErrorInner::MissingValue(_) => None,
            ErrorInner::EnvDeserialization { .. } => None,
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::UnsupportedFileFormat { .. } => None,
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::MissingFileExtension { .. } => None,
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::MissingRequiredFile { .. } => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.inner {
            ErrorInner::MissingValue(path) => {
                std::write!(f, "required configuration value is missing: '{}'", path)
            }
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::Io { path: Some(path), .. } => {
                std::write!(f,
                    "IO error occured while reading configuration file '{}'",
                    path.display(),
                )
            }
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::Io { path: None, .. } => {
                std::write!(f, "IO error occured while loading configuration")
            }
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::Deserialization { source: Some(source), .. } => {
                std::write!(f, "failed to deserialize configuration from {}", source)
            }
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::Deserialization { source: None, .. } => {
                std::write!(f, "failed to deserialize configuration")
            }
            ErrorInner::EnvDeserialization { field, key, msg } => {
                std::write!(f,
                    "failed to deserialize value `{}` from environment variable `{}`: {}",
                    field,
                    key,
                    msg,
                )
            }
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::UnsupportedFileFormat { path } => {
                std::write!(f,
                    "unknown configuration file format/extension: '{}'",
                    path.display(),
                )
            }
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::MissingFileExtension { path } => {
                std::write!(f,
                    "cannot guess configuration file format due to missing file extension in '{}'",
                    path.display(),
                )
            }
            #[cfg(any(feature = "toml", feature = "yaml"))]
            ErrorInner::MissingRequiredFile { path } => {
                std::write!(f,
                    "required configuration file does not exist: '{}'",
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
