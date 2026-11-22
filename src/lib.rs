//! Confique is a type-safe, layered, light-weight, `serde`-based configuration library.
//!
//! The core of the library is the [`Config`] trait and [its derive-macro][macro@Config].
//! You define your configuration value as one or more structs, each of which has
//! to `#[derive(Config)]`. Then you can use different ways of loading an instance
//! of your root configuration struct.
//!
//!
//! # How to use
//!
//! Add `confique` as dependency to your `Cargo.toml` and remember to enable the
//! crate features for file formats you are interested in. For example:
//! `cargo add confique --features=toml`.
//!
//! ## Defining your configuration with structs
//!
//! First, define some structs that describe all your configuration values. Use
//! the types you want to use in your code. For example, if you have a `port`
//! config and your code needs that value, it should be of type `u16`,
//! and *not* `Option<u16>` or `String`. That way, the code using that value is
//! cleanest.
//!
//! Small example:
//!
//! ```
//! use confique::Config;
//!
//! #[derive(Config)]
//! struct Conf {
//!     // A required value. Since it's not `Option<_>`, it has to be specified when
//!     // loading the configuration, or else loading returns an error.
//!     username: String,
//!
//!     // An optional value.
//!     welcome_message: Option<String>,
//!
//!     // A required value with default value. If no other value is specified
//!     // (e.g. in a config file), the default value is used.
//!     #[config(default = 8080)]
//!     port: u16,
//! }
//! # fn main() {}
//! ```
//!
//! As your application grows, oftentimes you want to split the configuration
//! into multiple structs. This has the added benefit that your config files
//! are somewhat structured or have sections. You can do that by including
//! other types that implement `Config` with `#[config(nested)]`.
//!
//! ```
//! use std::path::PathBuf;
//! use confique::Config;
//!
//! #[derive(Config)]
//! struct Conf {
//!     username: String,
//!
//!     #[config(nested)]
//!     log: LogConf,
//!
//!     #[config(nested)]
//!     db: DbConf,
//! }
//!
//! #[derive(Config)]
//! struct LogConf {
//!     #[config(default = true)]
//!     stdout: bool,
//!
//!     file: Option<PathBuf>,
//! }
//!
//! #[derive(Config)]
//! struct DbConf {
//!     // ...
//! }
//! # fn main() {}
//! ```
//!
//! You can also attach some other attributes to fields. For example, with
//! `#[config(env = "KEY")]`, you can load a value from an environment variable.
//! With `#[config(validate = ...)]` you can add validation checks. For more
//! information, see the [docs for the derive macro][macro@Config].
//!
//! Note: if a field has `#[config(nested)]`, its type must implement
//! [`Config`], otherwise it must implement `serde::Deserialize`.
//!
//!
//! ## Loading the configuration
//!
//! Here, you have multiple options. Most of the time, you can probably use the
//! provided high-level methods of [`Config`], like [`Config::from_file`] and
//! [`Config::builder`].
//!
//! ```
//! use confique::Config;
//!
//! # #[derive(Config)]
//! # struct Conf {}
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load from a single file only.
//! # #[cfg(feature = "toml")]
//! let config = Conf::from_file("config.toml")?;
//!
//! // Or load from multiple sources (higher priority sources are listed first).
//! # #[cfg(feature = "toml")]
//! let config = Conf::builder()
//!     .env()
//!     .file("config.toml")
//!     .file("/etc/myapp/config.toml")
//!     .load()?;
//! # Ok(())
//! # }
//! ```
//!
//! But you can also assemble your configuration yourself. That's what
//! the *layer* types are for (i.e. [`Config::Layer`]), which the derive
//! generates next to your struct as `{Name}Layer`. These implement
//! `serde::Deserialize` and can thus be loaded from a vast number of sources.
//! One of those sources is the built-in [`File`] which gives you a bit more
//! control when loading configuration from files. And you can always simply
//! create an instance of the layer type in Rust code, either with struct
//! initializer syntax or, more conveniently, with the [`layer!`] macro.
//!
//! If you just want to add such a layer to the usual sources (e.g. values from
//! CLI arguments), pass it to [`Builder::layer`]:
//!
//! ```no_run
//! # #[cfg(not(feature = "toml"))]
//! # fn main() {}
//! # #[cfg(feature = "toml")]
//! # fn main() -> Result<(), confique::Error> {
//! use confique::Config;
//!
//! #[derive(Config)]
//! struct Conf {
//!     foo: f32,
//! }
//!
//! let cli_layer = ConfLayer { foo: Some(3.14) };
//! let config = Conf::builder()
//!     .layer(cli_layer)
//!     .env()
//!     .file("config.toml")
//!     .load()?;
//! # Ok(())
//! # }
//! ```
//!
//! For full control, you can also merge layers yourself: combine
//! them via [`Layer::with_fallback`] and convert them to the actual config
//! type via [`Config::from_layer`]. And you probably also want to use
//! [`Layer::default_values`] as the last layer.
//!
//! ```no_run
//! # #[cfg(not(feature = "toml"))]
//! # fn main() {}
//! # #[cfg(feature = "toml")]
//! # fn main() -> Result<(), confique::Error> {
//! use confique::{Config, File, FileFormat, Layer};
//!
//! #[derive(Config)]
//! struct Conf {
//!     foo: f32,
//! }
//!
//! let from_file: ConfLayer = File::with_format("/etc/foo/config", FileFormat::Toml)
//!     .required()
//!     .load()?;
//! let manual = ConfLayer { foo: Some(3.14) };
//! let defaults = ConfLayer::default_values();
//!
//! let merged = from_file.with_fallback(manual).with_fallback(defaults);
//! let config = Conf::from_layer(merged)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using your configuration
//!
//! Well, this is the simple part: the loaded configuration is just an instance
//! of your struct. And you already know how to access fields of structs!
//!
//!
//! # Other notes
//!
//! - To use CLI as a layer when loading your configuration, see the `clap`
//!   example!
//!
//! # Cargo features
//!
//! This crate has a Cargo feature for each supported file format. These are not
//! enabled by default, so you have to specify which file formats you are
//! interested in.
//!
//! ```toml
//! confique = { version = "...", features = ["toml"] }
//! ```
//!
//! All crate features:
//!
//! - `toml`: enables TOML support and adds the `toml` dependency.
//! - `yaml`: enables YAML support and adds the `serde_yaml` dependency.
//! - `json5`: enables JSON5 support and adds the `json5` dependency.

use serde::Deserialize;

#[doc(hidden)]
pub mod internal;

mod builder;
pub mod env;
mod error;
pub mod meta;

#[cfg(any(feature = "toml", feature = "yaml", feature = "json5"))]
mod file;

#[cfg(any(feature = "toml", feature = "yaml", feature = "json5"))]
mod template;

#[cfg(feature = "json5")]
pub mod json5;

#[cfg(feature = "toml")]
pub mod toml;

#[cfg(feature = "yaml")]
pub mod yaml;

#[cfg(test)]
mod test_utils;

// The derive macro emits paths starting with `confique::`.
#[cfg(doc)]
extern crate self as confique;

#[cfg(doc)]
pub mod _example;

#[cfg(any(doc, doctest))]
pub mod _derive;


pub use serde;
pub use self::{
    builder::Builder,
    error::Error,
};

#[cfg(any(feature = "toml", feature = "yaml", feature = "json5"))]
pub use crate::{
    file::{File, FileFormat},
    template::FormatOptions,
};


/// Derives (automatically implements) [`Config`] for a struct.
///
/// For details, see [`_derive`] or [`_example`].
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
///
///     #[config(default = ["x-user", "x-password"])]
///     headers: Vec<String>,
/// }
/// # fn main() {}
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
pub use confique_macro::Config;

/// Creates a configuration layer directly in code.
///
/// This is a convenient alternative to building a layer with struct literal
/// syntax (where all values have to be `Some(_)` of the exact field type), or
/// to writing a config file inside your Rust code. It is mostly useful for
/// tests or for providing a layer with fixed values.
///
/// ```
/// use std::{env::temp_dir, net::IpAddr, path::PathBuf};
/// use confique::Config;
///
/// #[derive(Config)]
/// struct Conf {
///     #[config(nested)]
///     http: HttpConf,
///     log_file: PathBuf,
/// }
///
/// #[derive(Config)]
/// struct HttpConf {
///     #[config(default = 8080)]
///     port: u16,
///     bind: IpAddr,
///     headers: Vec<String>,
/// }
///
/// # fn main() -> Result<(), confique::Error> {
/// let layer = confique::layer!(Conf {
///     http: {                                 // nested configuration
///         bind: "127.0.0.1",                  // string deserialized into `IpAddr`
///         headers: ["x-user", "x-password"],  // array deserialized into `Vec<String>`
///     },
///     log_file: #(temp_dir().join("app.log")), // raw Rust value of exact type
/// })?;
///
/// assert_eq!(layer.http.port, None); // default values are not applied
/// assert_eq!(layer.http.bind, Some(IpAddr::from([127, 0, 0, 1])));
/// assert_eq!(layer.http.headers, Some(vec!["x-user".into(), "x-password".into()]));
/// assert_eq!(layer.log_file, Some(temp_dir().join("app.log")));
/// # Ok(())
/// # }
/// ```
///
/// The macro returns `Result<<C as Config>::Layer, Error>`. Fields that are not
/// mentioned are `None`/empty, i.e. default values are *not* applied. To get
/// your actual configuration, use the layer like any other, e.g. pass it to
/// [`Builder::layer`], or combine it with other layers via
/// [`Layer::with_fallback`] and [`Layer::default_values`]. If you only need
/// the default values as fallback, use [`config!`] instead.
///
///
/// # Syntax
///
/// Inside the braces, there is a comma separated list of entries. Each entry
/// has the form `field: value` where `value` is one of:
///
/// - **An expression**, e.g. `"127.0.0.1"`, `8080` or `my_var`. The value is
///   *deserialized* into the field, just like a value from a configuration
///   file. That means the value does not need to have the exact type of the
///   field, e.g. a string can be used for an `IpAddr` or `PathBuf` field.
///   Field validators and `deserialize_with` are applied as well. The
///   expression needs to implement [`IntoDeserializer`], which is the case
///   for all primitive types, strings, `Vec`, `HashMap`, `BTreeMap` and some
///   more.
/// - **An array**, e.g. `["foo", "bar"]`, deserialized as a sequence. Items
///   can be any expression implementing [`IntoDeserializer`], or again an
///   array or map, e.g. `[[1, 2], [3]]`. Items can have different types.
/// - **A map**, e.g. `{ "foo": 1, "bar": 2 }`, deserialized as a map
///   (preserving the order of entries). Keys need to be literals, values can
///   be anything that array items can be.
/// - **A nested configuration**, e.g. `{ port: 8080, bind: "::1" }`, using the
///   same syntax recursively. Distinguished from a map by the field names
///   being identifiers, not literals.
/// - **A raw Rust value** `#(expr)`, e.g. `#(PathBuf::from("/tmp"))`. The
///   expression is assigned as is (i.e. `Some(expr)`) and needs to have the
///   exact type of the field. This is *not* deserialized, so field validators
///   and `deserialize_with` are not applied! Use [`Layer::validate`] if you
///   need to run field validators. For a nested configuration, the expression
///   needs to be its layer, which replaces the whole nested layer.
///
/// Integer literals without type suffix are `i32` as usual, but are converted
/// to the field's type when deserialized (failing if the value doesn't fit).
/// For values outside the `i32` range, add a suffix, e.g. `5_000_000_000u64`.
///
///
/// # Errors
///
/// Misspelled or duplicate field names result in compile errors. An error is
/// returned at runtime if a value cannot be deserialized into its field,
/// including when a field validator fails.
///
/// Note that this macro accesses the fields of the layer types, which have
/// the same visibility as your configuration struct. So you can only use
/// this macro where your configuration type is visible.
///
/// [`IntoDeserializer`]: crate::serde::de::IntoDeserializer
pub use confique_macro::layer;

/// Creates a configuration directly in code, using default values for all
/// fields not specified.
///
/// Uses the same syntax as [`layer!`] and is a shorthand for creating a layer
/// with it, adding [`Layer::default_values`] as fallback and calling
/// [`Config::from_layer`].
///
/// ```
/// use std::net::IpAddr;
/// use confique::Config;
///
/// #[derive(Config)]
/// struct Conf {
///     #[config(default = 8080)]
///     port: u16,
///     bind: IpAddr,
/// }
///
/// # fn main() -> Result<(), confique::Error> {
/// let conf = confique::config!(Conf { bind: "127.0.0.1" })?;
/// assert_eq!(conf.port, 8080);
/// assert_eq!(conf.bind, IpAddr::from([127, 0, 0, 1]));
/// # Ok(())
/// # }
/// ```
///
/// Returns `Result<C, Error>`. In addition to the errors of `layer!`, this
/// fails if a required value is neither specified nor has a default value, or
/// if a struct validator fails.
pub use confique_macro::config;


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
    /// A layer of `Self` (a potentially partial configuration).
    ///
    /// This type has the exact same fields as `Self`, but with every field
    /// being optional. Its main use is to have a layered configuration from
    /// multiple sources where each layer might not contain all required
    /// values. The only thing that matters is that combining all layers will
    /// result in a configuration object that has all required values defined.
    type Layer: Layer;

    /// A description of this configuration.
    ///
    /// This is a runtime representation from the struct definition of your
    /// configuration type.
    const META: meta::Meta;

    /// Tries to create `Self` from a layer and validates itself.
    ///
    /// An [`Error`] is returned if:
    /// - any required values are not defined in `layer`, or
    /// - the struct validation fails (see `validate` attribute on derive macro)
    fn from_layer(layer: Self::Layer) -> Result<Self, Error>;

    /// Convenience builder to configure, load and merge multiple configuration
    /// sources. **Sources specified earlier have a higher priority**; later
    /// sources only fill in the gaps. After all sources have been loaded, the
    /// default values (usually specified with `#[config(default = ...)]`) are
    /// merged (with the lowest priority).
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
    /// #[cfg(feature = "toml")]
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
    #[cfg(any(feature = "toml", feature = "yaml", feature = "json5"))]
    fn from_file(path: impl Into<std::path::PathBuf>) -> Result<Self, Error> {
        let default_values = Self::Layer::default_values();
        let mut file = File::new(path)?;
        if !default_values.is_complete() {
            file = file.required();
        }

        Self::from_layer(file.load::<Self::Layer>()?.with_fallback(default_values))
    }
}

/// A configuration layer: all fields are optional. Can be directly deserialized
/// via `serde`.
pub trait Layer: for<'de> Deserialize<'de> {
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
    /// `None`. If it is set and non-empty, but it failed to deserialize into
    /// the target type, an error is returned. If set to an empty string *and*
    /// if it fails to deserialize, it's treated as not set.
    fn from_env() -> Result<Self, Error>;

    /// Combines two layers. `self` has a higher priority; missing values in
    /// `self` are filled with values in `fallback`, if they exist. The
    /// semantics of this method is basically like in [`Option::or`].
    fn with_fallback(self, fallback: Self) -> Self;

    /// Returns `true` if all values are unspecified/`None`.
    fn is_empty(&self) -> bool;

    /// Returns `true` if all required (non-optional) values in this
    /// configuration are set. If this returns `true`, [`Config::from_layer`]
    /// will not return an error.
    fn is_complete(&self) -> bool;

    /// Runs all field validators (i.e. set via `#[config(validate = ...)]` when
    /// deriving `Config`) on the values present in this layer, including
    /// nested layers. Fields that are `None` are skipped. Returns the first
    /// validation error.
    ///
    /// Field validators already run when a layer is deserialized (from files,
    /// environment variables or default values), so you usually don't need
    /// to call this. It is useful for layers you construct manually (e.g. from
    /// command line arguments) before passing them to
    /// [`Builder::layer`]. Struct validators are not run by this, as
    /// they require the complete configuration (see [`Config::from_layer`]).
    ///
    /// The default implementation does nothing and returns `Ok(())`.
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}
