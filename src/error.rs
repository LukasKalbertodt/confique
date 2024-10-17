use std::fmt;

use std::path::PathBuf;



/// Type describing all errors that can occur in this library.
///
/// *Note*: the `Display` and `Debug` impls of this type do not include
///  information about `Error::source` by default. When showing this error to
///  end users, you should traverse the `source`-chain and print each error.
///  Crates like `anyhow` and `eyre` do this for you. As a convenience feature,
///  you can use the "alternate" flag `#` when printing this error to include
///  the source, e.g. `println!("{:#}", err)`. This will only print the direct
///  source though, so a proper traversal is still preferred!
pub struct Error {
    pub(crate) inner: Box<ErrorInner>,
}

// If all these features are disabled, lots of these errors are unused. But
// instead of repeating this cfg-attribute a lot in the rest of the file, we
// just live with these unused variants. It's not like we need to optimize the
// size of `ErrorInner`.
#[cfg_attr(
    not(any(feature = "toml", feature = "yaml", feature = "json5")),
    allow(dead_code)
)]
pub(crate) enum ErrorInner {
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

    /// When the env variable `key` is not Unicode.
    EnvNotUnicode { field: String, key: String },

    /// When deserialization via `env` fails. The string is what is passed to
    /// `serde::de::Error::custom`.
    EnvDeserialization {
        field: String,
        key: String,
        msg: String,
    },

    /// When a custom `parse_env` function fails.
    EnvParseError {
        field: String,
        key: String,
        err: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Returned by the [`Source`] impls for `Path` and `PathBuf` if the file
    /// extension is not supported by confique or if the corresponding Cargo
    /// feature of confique was not enabled.
    UnsupportedFileFormat { path: PathBuf },

    /// Returned by the [`Source`] impls for `Path` and `PathBuf` if the path
    /// does not contain a file extension.
    MissingFileExtension { path: PathBuf },

    /// A file source was marked as required but the file does not exist.
    MissingRequiredFile { path: PathBuf },

    /// When a struct validation function fails.
    Validation { msg: String },
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &*self.inner {
            ErrorInner::Io { err, .. } => Some(err),
            ErrorInner::Deserialization { err, .. } => Some(&**err),
            ErrorInner::MissingValue(_) => None,
            ErrorInner::EnvNotUnicode { .. } => None,
            ErrorInner::EnvDeserialization { .. } => None,
            ErrorInner::EnvParseError { err, .. } => Some(&**err),
            ErrorInner::UnsupportedFileFormat { .. } => None,
            ErrorInner::MissingFileExtension { .. } => None,
            ErrorInner::MissingRequiredFile { .. } => None,
            ErrorInner::Validation { .. } => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.inner {
            ErrorInner::MissingValue(path) => {
                std::write!(f, "required configuration value is missing: '{path}'")
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
            ErrorInner::Deserialization { source: Some(source), err } => {
                std::write!(f, "failed to deserialize configuration from {source}")?;
                if f.alternate() {
                    f.write_str(": ")?;
                    fmt::Display::fmt(&err, f)?;
                }
                Ok(())
            }
            ErrorInner::Deserialization { source: None, err } => {
                std::write!(f, "failed to deserialize configuration")?;
                if f.alternate() {
                    f.write_str(": ")?;
                    fmt::Display::fmt(&err, f)?;
                }
                Ok(())
            }
            ErrorInner::EnvNotUnicode { field, key } => {
                std::write!(f, "failed to load value `{field}` from \
                    environment variable `{key}`: value is not valid unicode")
            }
            ErrorInner::EnvDeserialization { field, key, msg } => {
                std::write!(f, "failed to deserialize value `{field}` from \
                    environment variable `{key}`: {msg}")
            }
            ErrorInner::EnvParseError { field, key, err } => {
                std::write!(f, "failed to parse environment variable `{key}` into \
                    field `{field}`")?;
                if f.alternate() {
                    f.write_str(": ")?;
                    fmt::Display::fmt(&err, f)?;
                }
                Ok(())
            }
            ErrorInner::UnsupportedFileFormat { path } => {
                std::write!(f,
                    "unknown configuration file format/extension: '{}'",
                    path.display(),
                )
            }
            ErrorInner::MissingFileExtension { path } => {
                std::write!(f,
                    "cannot guess configuration file format due to missing file extension in '{}'",
                    path.display(),
                )
            }
            ErrorInner::MissingRequiredFile { path } => {
                std::write!(f,
                    "required configuration file does not exist: '{}'",
                    path.display(),
                )
            }
            ErrorInner::Validation { msg } => {
                std::write!(f, "config validation failed: {msg}")
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
