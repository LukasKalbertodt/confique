//! Types implementing [`Source`], thus representing some source of
//! configuration values.

use std::{ffi::OsStr, fs, io, path::{Path, PathBuf}};

use crate::{Config, Error, ErrorInner, Partial, Source};



impl<C: Config> Source<C> for &Path {
    fn load(&self) -> Result<C::Partial, Error> {
        let ext = self.extension().ok_or_else(|| {
            ErrorInner::MissingFileExtension { path: self.into() }
        })?;
        let format = FileFormat::from_extension(ext).ok_or_else(|| {
            ErrorInner::UnsupportedFileFormat { extension: ext.into(), path: self.into() }
        })?;

        <File as Source<C>>::load(&File::new(self, format))
    }
}

impl<C: Config> Source<C> for PathBuf {
    fn load(&self) -> Result<C::Partial, Error> {
        <&Path as Source<C>>::load(&&**self)
    }
}

pub struct File {
    path: PathBuf,
    format: FileFormat,
    required: bool,
}

impl File {
    pub fn new(path: impl Into<PathBuf>, format: FileFormat) -> Self {
        Self {
            path: path.into(),
            format,
            required: false,
        }
    }

    #[cfg(feature = "toml")]
    pub fn toml(path: impl Into<PathBuf>) -> Self {
        Self::new(path, FileFormat::Toml)
    }

    #[cfg(feature = "yaml")]
    pub fn yaml(path: impl Into<PathBuf>) -> Self {
        Self::new(path, FileFormat::Yaml)
    }

    /// Marks this file as required, meaning that `<File as Source<_>>::load`
    /// will return an error if the file does not exist. Otherwise, an empty
    /// layer (all values are `None`) is returned.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

impl<C: Config> Source<C> for File {
    // Unfortunately, if no file format is enabled, this emits unused variable
    // warnings. This should not happen as `self`, a type containing an empty
    // enum, is in scope, meaning that the code cannot be reached.
    #[cfg_attr(
        not(any(feature = "toml", feature = "yaml")),
        allow(unused_variables),
    )]
    fn load(&self) -> Result<C::Partial, Error> {
        // Load file contents. If the file does not exist and was not marked as
        // required, we just return an empty layer.
        let file_content = match fs::read(&self.path) {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::NotFound && !self.required => {
                return Ok(C::Partial::empty());
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

pub enum FileFormat {
    #[cfg(feature = "toml")] Toml,
    #[cfg(feature = "yaml")] Yaml,
}

impl FileFormat {
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
