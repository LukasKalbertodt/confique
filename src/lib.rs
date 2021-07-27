use std::path::PathBuf;

use serde::Deserialize;

#[doc(hidden)]
pub mod internal;

mod builder;
mod env;
mod error;
mod file;
mod format;
pub mod meta;

#[cfg(feature = "toml")]
pub mod toml;

#[cfg(feature = "yaml")]
pub mod yaml;


pub use serde;
pub use self::{
    builder::Builder,
    error::Error,
    file::{File, FileFormat},
};


/// Derives (automatically implements) [`Config`] for a struct.
///
/// This only works for structs with named fields, but not for tuple structs,
/// unit structs, enums, or unions.
///
/// # Quick example
///
/// ```
/// use confique::Config;
/// use std::net::IpAddr;
///
/// #[derive(Config)]
/// struct Conf {
///     color: Option<String>,
///
///     #[config(nested)]
///     http: HttpConf,
/// }
///
/// #[derive(Config)]
/// struct HttpConf {
///     #[config(env = "APP_PORT")]
///     port: u16,
///
///     #[config(default = "127.0.0.1")]
///     bind: IpAddr,
/// }
/// ```
///
/// This derives `Config` for the two structs.
///
/// - `HttpConf::port` can be loaded from the environment variable `APP_PORT`.
/// - `HttpConf::bind` has a default value of `127.0.0.1` (the string is turned
///   into the `IpAddr` via its `Deserialize` impl). Thus a value for this
///   field does not need to be present when loading configuration.
/// - `Conf::color` is optional and does not need to be present when loading the
///   configuration.
///
///
/// # How to use
///
/// There are two types of fields distinguished by this macro: nested and leaf
/// fields.
///
/// - **Nested fields**: they have to be annotated with `#[config(nested)]` and
///   contain a nested configuration object. The type of this field must
///   implement `Config`. As implied by the previous statement, `Option<_>` as
///   type for nested fields is not allowed.
///
/// - **Leaf fields**: all fields *not* annotated with `#[config
///   (nested)]`, these contain your actual values. The type of such a field
///   has to implement `serde::Deserialize`.
///
/// Doc comments on the struct and the individual fields are interpreted and
/// stored in [`Meta`]. They are used in the formatting functions
/// (e.g. `toml::format`).
///
///
/// ## Attributes
///
/// This macro currently recognizes the following attributes for leaf fields:
///
/// - **`#[config(default = ...)]`**: sets a default value for this field. This
///   is returned by [`Partial::default_values`] and, in most circumstances,
///   used as a last "layer" to pull values from that have not been set in a
///   layer of higher-priority. Currently, Boolean, float, integer and string
///   values are allowed.
///
/// - **`#[config(env = "KEY")]`**: assigns an environment variable to this
///   field. In [`Partial::from_env`], the variable is checked and
///   deserialized into the field if present.
///
///
/// ## Special types for leaf fields
///
/// These types give a different meaning/semantic to the field. Please note that
/// due to the limitations of derive macros, the type is checked *literally*.
/// So it won't work if you rename symbols or used full paths.
///
/// - **`Option<T>`**: this marks the field as an optional field. All other
///   fields are non-optional and will raise an error if while loading the
///   configuration, no value has been set for them.
///
///
///
/// # What the macro generates
///
/// This macro emits one `impl confique::Config for … { … }` block. But in order
/// to implement that trait, a *partial type* of your struct is also generated.
/// That partial type lives in its own module and derives
/// `serde::Deserialize`.
///
/// The example in the "Quick example" section above would expand to something
/// like this:
///
/// ```ignore
/// impl confique::Config for Conf {
///     type Partial = confique_partial_conf::PartialConf;
///     ...
/// }
/// mod confique_partial_conf {
///     #[derive(serde::Deserialize)]
///     pub(super) struct PartialConf {
///         pub(super) color: Option<String>,
///
///         #[serde(default = "confique::Partial::empty")]
///         pub(super) http: <HttpConf as confique::Config>::Partial,
///     }
///
///     impl confique::Partial for PartialConf { ... }
/// }
///
/// impl confique::Config for HttpConf {
///     type Partial = confique_partial_http_conf::PartialHttpConf;
///     ...
/// }
/// mod confique_partial_http_conf {
///     #[derive(serde::Deserialize)]
///     pub(super) struct PartialHttpConf {
///         pub(super) port: Option<u16>,
///         pub(super) bind: Option<IpAddr>,
///     }
///
///     impl confique::Partial for PartialHttpConf { ... }
/// }
/// ```
pub use confique_macro::Config;


/// A configuration object that can be deserialized in layers via `serde`.
///
/// You would usually derive this trait for your own type and then load the
/// configuration with one of the provided methods, like
/// [`from_file`][Self::from_file] or [`builder`](Self::builder).
///
/// # Deriving
///
/// This trait is usually derived as implementing it manually usually entails
/// writing some repetitive boilerplate code, that goes against the "don't
/// repeat yourself" principle. See [the documentation of the derive
/// macro][macro@Config] for more information!
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
    /// # Example
    ///
    /// In the following example, configuration is first loaded from environment
    /// variables, then from `app.toml`, then from `/etc/app/config.toml` and
    /// finally from the configured default values. Values found earlier in
    /// this list have precedence.
    ///
    /// ```
    /// use confique::Config;
    ///
    /// #[derive(Config)]
    /// struct Conf {
    ///     #[config(env = "APP_PORT", default = 8080)]
    ///     port: u16,
    /// }
    ///
    /// let conf = Conf::builder()
    ///     .env()
    ///     .file("app.toml")
    ///     .file("/etc/app/config.toml")
    ///     .load();
    /// ```
    fn builder() -> Builder<Self> {
        Builder::new()
    }


    /// Load the configuration from a single file.
    ///
    /// If you rather want to load from multiple sources, use
    /// [`Config::builder`]. Infers the file format from the file extension.
    /// Returns an error in these cases:
    ///
    /// - The path does not have a known file extension.
    /// - Loading the file fails.
    /// - The file does not specify all required configuration values.
    ///
    /// # Example
    ///
    /// ```
    /// use confique::Config;
    ///
    /// #[derive(Config)]
    /// struct Conf {
    ///     port: u16,
    /// }
    ///
    /// let conf = Conf::from_file("config.toml");
    /// ```
    fn from_file(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let default_values = Self::Partial::default_values();
        let mut file = File::new(path)?;
        if !default_values.is_complete() {
            file = file.required();
        }

        Self::from_partial(file.load::<Self::Partial>()?.with_fallback(default_values))
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

    /// Loads values from environment variables. This is only relevant for
    /// fields annotated with `#[config(env = "...")]`: all fields not
    /// annotated `env` will be `None`.
    ///
    /// If the env variable corresponding to a field is not set, that field is
    /// `None`. If it is set but it failed to deserialize into the target type,
    /// an error is returned.
    fn from_env() -> Result<Self, Error>;

    /// Combines two partial configuration objects. `self` has a higher
    /// priority; missing values in `self` are filled with values in `fallback`,
    /// if they exist. The semantics of this method is basically like in
    /// [`Option::or`].
    fn with_fallback(self, fallback: Self) -> Self;

    /// Returns `true` if all values are unspecified/`None`.
    fn is_empty(&self) -> bool;

    /// Returns `true` if all required (non-optional) values in this
    /// configuration are set. If this returns `true`, `Config::from_partial`
    /// will not return an error.
    fn is_complete(&self) -> bool;
}
