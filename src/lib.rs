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
//! Note: if a field hast `#[config(nested)]`, its type must implement
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
//! the *layer* types are for (i.e. [`Config::Layer`]). These implement
//! `serde::Deserialize` and can thus be loaded from a vast number of sources.
//! One of those sources is the built-in [`File`] which gives you a bit more
//! control when loading configuration from files. And you can always simply
//! create an instance of the layer type by writing all values in Rust code
//! with struct initializer syntax!
//!
//! Once you have all your layers collected, you have to combine
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
//! type ConfLayer = <Conf as Config>::Layer;
//! let from_file: ConfLayer = File::with_format("/etc/foo/config", FileFormat::Toml)
//!     .required()
//!     .load()?;
//! let manual = ConfLayer {
//!     // Remember: all fields in the layer types are `Option`s!
//!     foo: Some(3.14),
//! };
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
/// This only works for structs with named fields (i.e. not for tuple structs,
/// unit structs, enums, or unions). This macro only works sometimes inside of
/// functions (as it generates a module and symbol resolution is weird in that
/// case); if you get weird errors "symbol not found", just move the struct
/// definition outside of the function.
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
/// - **Leaf fields**: all fields *not* annotated with `#[config(nested)]`,
///   these contain your actual values. The type of such a field has to
///   implement `serde::Deserialize` or you have to add a `deserialize_with`
///   attribute.
///
/// Doc comments on the struct and the individual fields are interpreted and
/// stored in [`Meta`][meta::Meta]. They are used in the formatting functions
/// (e.g. `toml::format`).
///
/// ## Special types for leaf fields
///
/// These types give a different meaning/semantic to the field. Please note that
/// due to the limitations of derive macros, the type is checked *literally*.
/// So it won't work if you rename symbols or use full paths.
///
/// - **`Option<T>`**: this marks the field as an optional field. All other
///   fields are non-optional and will raise an error if while loading the
///   configuration, no value has been set for them. Optional fields cannot have
///   a `#[config(default = ...)]` attribute as that would not make sense. Note:
///   the (unqualified) symbol `Option` must be in scope and refer to
///   `std::option::Option`.
///
///
/// ## Field Attributes
///
/// The following attributes can be attached to struct fields.
///
/// ### `default`
///
/// ```ignore
/// #[config(default = ...)]
/// ```
///
/// Sets a default value for this field. This is returned by
/// [`Layer::default_values`] and, in most circumstances, used as a
/// last "layer" to pull values from that have not been set in a layer of
/// higher-priority. Currently, the following expressions are allowed:
///
/// - Booleans, e.g. `default = true`
/// - Integers, e.g. `default = 900`
/// - Floats, e.g. `default = 3.14`
/// - Strings, e.g. `default = "fox"`
/// - Arrays, e.g. `default = ["foo", "bar"]`
/// - Key value maps, e.g. `default = { "cat": 3.14, "bear": 9.0 }`
///
/// Map keys can be Booleans, integers, floats, and strings. For array and map
/// values, you can use any of the expressions in the list above (i.e. you
/// can nest arrays/maps).
///
/// The field value is deserialized from the specified default value
/// (via `serde::de::IntoDeserializer`). So the expression after `default =`
/// is often not the same Rust type as your field. For example, you can have
/// `#[config(default = "/foo/bar")]` on the field `path: PathBuf`. This
/// works fine as `PathBuf` can be deserialized from a string. (Also see the
/// `IpAddr` field in the example above.)
///
/// If you use an integer or float literal without type suffix, `confique` has
/// to infer the exact type from the type of the field. This should work in
/// most cases (`u8`, `f32`, `Vec<i16>`, `[f64; 3]`, ...), but this type
/// inference is very basic, not even close to what Rust can do. If confique
/// cannot figure out the type, it defaults to `i32` for integers and `f64`
/// for floats (like Rust does). If that causes problems for you, just add a
/// type suffix, e.g. `default = 800u32`.
///
/// ### `env`
///
/// ```ignore
/// #[config(env = "KEY")]
/// ```
///
/// Assigns an environment variable to this field. In [`Layer::from_env`], the
/// variable is checked and deserialized into the field if present.
///
/// If the env var is set to an empty string and if the field fails to
/// parse/deserialize/validate, it is treated as unset.
///
/// ### `parse_env`
///
/// ```ignore
/// #[config(parse_env = path::to::function)]
/// ```
///
/// Function used to parse environment variables. Mostly useful if you need to
/// parse lists or other complex objects from env vars. Function needs
/// signature `fn(&str) -> Result<T, impl std::error::Error>` where `T` is the
/// type of the field. Can only be present if the `env` attribute is present.
/// Also see [`env::parse`].
///
/// #### `deserialize_with`
///
/// ```ignore
/// #[config(deserialize_with = path::to::function)]
/// ```
///
/// Like [serde's `deserialize_with` attribute][serde-deser].
///
/// [serde-deser]: https://serde.rs/field-attrs.html#deserialize_with
///
/// #### `validate`
///
/// ```ignore
/// #[config(validate = path::to::function)]
/// // or
/// #[config(validate(<expr>, "msg"))]
/// ```
///
/// Adds a validation to the field, i.e. a check that must succeed to be able to
/// load the configuration. The validator is called as part of the
/// deserialization, and is thus executed for all layers, not just for the
/// merged configuration. The attribute can be specified multiple times.
///
/// > *Note*: remember ["Parse, don't validate"][parse-not-validate]! If you can
///    reasonably represent your validation logic as a type, you should use
///    that type instead of validating a weakly-typed field. Example: if your
///    config value is an IP-address, use the dedicated `std::net::IpAddr` as
///    field type (can be deserialized from strings) instead of a `String`
///    field with a `validate` function making sure it's a valid IP-address.
/// >
/// > ```ignore
/// > // GOOD
/// > addr: std::net::IpAddr,
/// >
/// > // BAD
/// > #[config(validate(addr.parse::<std::net::IpAddr>().is_ok(), "not a valid IP-address"))]
/// > addr: String,
/// > ```
///
/// [parse-not-validate]: https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/
///
/// The `validate = path::to::function` syntax expects a function that is
/// callable as `Fn(&T) -> Result<(), E>` where `T` is the (non-optional) type
/// of the field and `E` can be any type implementing `fmt::Display` (e.g. just
/// `&str`). Example:
///
/// ```
/// use confique::Config;
///
/// #[derive(Config)]
/// struct Conf {
///     #[config(validate = is_valid_user)]
///     user: Option<String>,
/// }
///
/// fn is_valid_user(user: &String) -> Result<(), &'static str> {
///     if user == "root" {
///         return Err("user 'root' is not allowed");
///     }
///     if !user.is_ascii() {
///         return Err("user must be an ASCII string");
///     }
///     Ok(())
/// }
/// # fn main() {}
/// ```
///
/// The `validate(<expr>, "msg")` syntax is only for convenience and intended
/// for simple cases. It works similar to the `assert!` macro as it expects an
/// expression validating to `bool` and a string error message. The expression
/// can access the field value by reference via the field's name. If the
/// expression validates to `false`, this is treated as a validation error.
/// Examples:
///
/// ```
/// use confique::Config;
///
/// #[derive(Config)]
/// struct Conf {
///     #[config(
///         validate(!name.is_empty(), "name must not be empty"),
///         validate(name.is_ascii(), "name must be ASCII"),
///     )]
///     name: String,
///
///     #[config(validate(*port >= 1024, "cannot use ports < 1024 as non-root user"))]
///     port: Option<u16>,
/// }
/// ```
///
///
/// ## Struct attributes
///
/// The following attributes can be attached to the struct itself.
///
/// #### `validate`
///
/// ```ignore
/// #[config(validate = path::to::function)]
/// ```
///
/// Adds a validation to the config struct, i.e. a check that must suceed to be
/// able to load the configuration. The validator is called inside
/// `Config::from_layer`, i.e. only after all layers have been merged.
///
/// The given`path::to::function` is expected to be a function callable as
/// `Fn(&T) -> Result<(), E>` where `T` is the struct type (`Self`) and `E` can
/// be any type implementing `fmt::Display` (e.g. just `&str`). Example:
///
/// ```
/// use confique::Config;
///
/// #[derive(Config)]
/// #[config(validate = Self::validate)]
/// struct ColorMixConfig {
///     source_weight: f32,
///     target_weight: f32,
/// }
///
/// impl ColorMixConfig {
///     fn validate(&self) -> Result<(), &'static str> {
///         if self.source_weight + self.target_weight > 1.0 {
///             return Err("sum of weights must not exceed 1");
///         }
///         Ok(())
///     }
/// }
/// # fn main() {}
/// ```
///
/// ### `layer_attr`
///
/// ```ignore
/// #[config(layer_attr(...))]
/// ```
///
/// Specify attributes that should be attached to the layer struct definition.
/// For example, `#[config(layer_attr(derive(Clone)))]` can be used to make
/// the layer type implement `Clone`.
///
/// This attribute can also be applied to struct fields.
///
///
/// # What the macro generates
///
/// This macro emits one `impl confique::Config for … { … }` block. But in order
/// to implement that trait, a *layer type* of your struct is also generated.
/// That layer type lives in its own module and derives `serde::Deserialize`.
///
/// The example in the "Quick example" section above would expand to something
/// like this:
///
/// ```ignore
/// // ----- Generated for `Conf` -----
/// impl confique::Config for Conf {
///     type Layer = confique_conf_layer::ConfLayer;
///     ...
/// }
/// mod confique_conf_layer {
///     #[derive(serde::Deserialize)]
///     pub(super) struct ConfLayer {
///         pub(super) color: Option<String>,
///
///         #[serde(default = "confique::Layer::empty")]
///         pub(super) http: <HttpConf as confique::Config>::Layer,
///     }
///
///     impl confique::Layer for ConfLayer { ... }
/// }
///
/// // ----- Generated for `HttpConf` -----
/// impl confique::Config for HttpConf {
///     type Layer = confique_http_conf_layer::HttpConfLayer;
///     ...
/// }
/// mod confique_http_conf_layer {
///     #[derive(serde::Deserialize)]
///     pub(super) struct HttpConfLayer {
///         pub(super) port: Option<u16>,
///         pub(super) bind: Option<IpAddr>,
///         pub(super) headers: Option<Vec<String>>,
///     }
///
///     impl confique::Layer for HttpConfLayer { ... }
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
    /// A layer of `Self` (a potentially partial configuration).
    ///
    /// This type is supposed to have the exact same fields as this one, but
    /// with every field being optional. Its main use is to have a layered
    /// configuration from multiple sources where each layer might not contain
    /// all required values. The only thing that matters is that combining all
    /// layers will result in a configuration object that has all required
    /// values defined.
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
}
