use std::path::PathBuf;

use crate::{Config, Error, File, Partial};


/// Convenience builder to configure, load and merge multiple configuration
/// sources.
///
/// **Sources specified earlier have a higher priority**. Obtained via
/// [`Config::builder`].
pub struct Builder<C: Config> {
    sources: Vec<Source<C>>,
}

impl<C: Config> Builder<C> {
    pub(crate) fn new() -> Self {
        Self {
            sources: vec![],
        }
    }

    /// Adds a configuration file as source. Infers the format from the file
    /// extension. If the path has no file extension or the extension is
    /// unknown, [`Builder::load`] will return an error.
    ///
    /// The file is not considered required: if the file does not exist, an
    /// empty configuration (`C::Partial::empty()`) is used for this layer.
    pub fn file(mut self, path: impl Into<PathBuf>) -> Self {
        self.sources.push(Source::File(path.into()));
        self
    }

    /// Adds an already loaded partial configuration as source.
    pub fn preloaded(mut self, partial: C::Partial) -> Self {
        self.sources.push(Source::Preloaded(partial));
        self
    }

    /// Loads all configured sources in order. Earlier sources have a higher
    /// priority, later sources only fill potential gaps.
    ///
    /// Will return an error if loading the sources fails or if the merged
    /// configuration does not specify all required values.
    pub fn load(self) -> Result<C, Error> {
        let mut partial = C::Partial::empty();
        for source in self.sources {
            let layer = match source {
                Source::File(path) => File::new(path)?.load()?,
                Source::Preloaded(p) => p,
            };

            partial = partial.with_fallback(layer);
        }

        C::from_partial(partial.with_fallback(C::Partial::default_values()))
    }
}

enum Source<C: Config> {
    File(PathBuf),
    Preloaded(C::Partial),
}
