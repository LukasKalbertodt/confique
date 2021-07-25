use serde::Deserialize;

#[doc(hidden)]
pub mod internal;

mod builder;
mod error;
mod file;
pub mod meta;

#[cfg(feature = "toml")]
pub mod toml;


pub use serde;
pub use confique_macro::Config;
pub use self::{
    builder::Builder,
    error::Error,
    file::{File, FileFormat},
};



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

    /// A description of this configuration.
    ///
    /// This is a runtime representation from the struct definition of your
    /// configuration type.
    const META: meta::Meta;

    /// Tries to create `Self` from a potentially partial object.
    ///
    /// If any required values are not defined in `partial`, an [`Error`] is
    /// returned.
    fn from_partial(partial: Self::Partial) -> Result<Self, Error>;

    /// Convenience builder to configure, load and merge multiple configuration
    /// sources. **Sources specified earlier have a higher priority**; later
    /// sources only fill in the gaps. After all sources have been loaded, the
    /// default values (usually specified with `#[default = ...]`) are merged
    /// (with the lowest priority).
    ///
    /// TODO: Example
    fn builder() -> Builder<Self> {
        Builder::new()
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
