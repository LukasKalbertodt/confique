use std::{ffi::OsStr, fs, io, path::PathBuf};

use crate::{error::ErrorInner, Error, Layer};


/// A file as source for configuration.
///
/// By default, the file is considered optional, meaning that on [`File::load`],
/// if the file does not exist, [`Layer::empty()`][crate::Layer::empty] is
/// returned.
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
        let ext = path
            .extension()
            .ok_or_else(|| ErrorInner::MissingFileExtension { path: path.clone() })?;
        let format = FileFormat::from_extension(ext)
            .ok_or_else(|| ErrorInner::UnsupportedFileFormat { path: path.clone() })?;

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

    /// Attempts to load the file into the layer `L`.
    pub fn load<L: Layer>(&self) -> Result<L, Error> {
        // Load file contents. If the file does not exist and was not marked as
        // required, we just return an empty layer.
        let file_content = match fs::read(&self.path) {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if self.required {
                    return Err(ErrorInner::MissingRequiredFile { path: self.path.clone() }.into());
                } else {
                    return Ok(L::empty());
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
        let error = |err| {
            Error::from(ErrorInner::Deserialization {
                err,
                source: Some(format!("file '{}'", self.path.display())),
            })
        };

        match self.format {
            #[cfg(feature = "toml")]
            FileFormat::Toml => {
                let s = std::str::from_utf8(&file_content).map_err(|e| error(Box::new(e)))?;
                toml::from_str(s).map_err(|e| error(Box::new(e)))
            }

            #[cfg(feature = "yaml")]
            FileFormat::Yaml => serde_yaml::from_slice(&file_content)
                .map_err(|e| error(Box::new(e))),

            #[cfg(feature = "json5")]
            FileFormat::Json5 => {
                let s = std::str::from_utf8(&file_content).map_err(|e| error(Box::new(e)))?;
                json5::from_str(s).map_err(|e| error(Box::new(e)))
            }
        }
    }
}

/// All file formats supported by confique.
///
/// All enum variants are `#[cfg]` guarded with the respective crate feature.
pub enum FileFormat {
    #[cfg(feature = "toml")]
    Toml,
    #[cfg(feature = "yaml")]
    Yaml,
    #[cfg(feature = "json5")]
    Json5,
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

            #[cfg(feature = "json5")]
            "json5" | "json" => Some(Self::Json5),

            _ => None,
        }
    }
}
