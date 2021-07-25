use std::{ffi::OsStr, fs, io, path::PathBuf};

use crate::{Error, Partial, error::ErrorInner};


/// A file as source for configuration.
///
/// By default, the file is considered optional, meaning that on [`File::load`],
/// if the file does not exist, `Partial::empty()` is returned.
pub struct File {
    path: PathBuf,
    format: FileFormat,
    required: bool,
}

impl File {
    /// Configuration file with the given path. The format is inferred from the
    /// file extension. If the path does not have an extension or it is
    /// unknown, an error is returned.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let path = path.into();
        let ext = path.extension().ok_or_else(|| {
            ErrorInner::MissingFileExtension { path: path.clone() }
        })?;
        let format = FileFormat::from_extension(ext).ok_or_else(|| {
            ErrorInner::UnsupportedFileFormat { path: path.clone() }
        })?;

        Ok(Self::with_format(path, format))
    }

    /// Config file with specified file format.
    pub fn with_format(path: impl Into<PathBuf>, format: FileFormat) -> Self {
        Self {
            path: path.into(),
            format,
            required: false,
        }
    }

    /// Marks this file as required, meaning that [`File::load`] will return an
    /// error if the file does not exist. Otherwise, an empty layer (all values
    /// are `None`) is returned.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Attempts to load the file into the partial configuration `P`.
    pub fn load<P: Partial>(&self) -> Result<P, Error> {
        // Unfortunately, if no file format is enabled, this emits unused variable
        // warnings. This should not happen as `self`, a type containing an empty
        // enum, is in scope, meaning that the code cannot be reached.
        #![cfg_attr(
            not(any(feature = "toml", feature = "yaml")),
            allow(unused_variables),
        )]

        // Load file contents. If the file does not exist and was not marked as
        // required, we just return an empty layer.
        let file_content = match fs::read(&self.path) {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if self.required {
                    return Err(ErrorInner::MissingRequiredFile { path: self.path.clone() }.into());
                } else {
                    return Ok(P::empty());
                }
            }
            Err(e) => {
                return Err(ErrorInner::Io {
                    path: Some(self.path.clone()),
                    err: e,
                }.into());
            }
        };

        // Helper closure to create an error.
        let error = |err| Error::from(ErrorInner::Deserialization {
            err,
            source: Some(format!("file '{}'", self.path.display())),
        });

        match self.format {
            #[cfg(feature = "toml")]
            FileFormat::Toml => toml::from_slice(&file_content)
                .map_err(|e| error(Box::new(e))),

            #[cfg(feature = "yaml")]
            FileFormat::Yaml => serde_yaml::from_slice(&file_content)
                .map_err(|e| error(Box::new(e))),
        }
    }
}

/// All file formats supported by confique.
///
/// All enum variants are `#[cfg]` guarded with the respective crate feature.
pub enum FileFormat {
    #[cfg(feature = "toml")] Toml,
    #[cfg(feature = "yaml")] Yaml,
}

impl FileFormat {
    /// Guesses the file format from a file extension, returning `None` if the
    /// extension is unknown or if the respective crate feature is not enabled.
    pub fn from_extension(ext: impl AsRef<OsStr>) -> Option<Self> {
        match ext.as_ref().to_str()? {
            #[cfg(feature = "toml")]
            "toml" => Some(Self::Toml),

            #[cfg(feature = "yaml")]
            "yaml" | "yml" => Some(Self::Yaml),

            _ => None,
        }
    }
}
