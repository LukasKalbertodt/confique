//! JSON5 specific features. This module only exists if the Cargo feature
//! `json5` is enabled.

use std::fmt::{self, Write};

use crate::{
    format::{self, ConfigFormatter},
    meta::Expr,
    Config,
};

// JSON5 serialization requires serde_json to convert Config -> serde_json::Value
#[cfg(all(feature = "serialize", not(feature = "serde_json")))]
compile_error!(
    "JSON5 serialization requires the `serde_json` feature. \
     Enable it with: features = [\"json5\", \"serialize\", \"serde_json\"]"
);

#[cfg(all(feature = "serialize", feature = "serde_json"))]
use serde::Serialize;

#[cfg(all(feature = "serialize", feature = "serde_json"))]
use crate::format::ValueAccess;

/// Options for generating a JSON5 template.
#[non_exhaustive]
pub struct FormatOptions {
    /// Indentation per level. Default: 2.
    pub indent: u8,

    /// Non JSON5-specific options.
    pub general: format::FormatOptions,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            general: Default::default(),
        }
    }
}

/// Formats the configuration description as a JSON5 file.
///
/// This can be used to generate a template file that you can give to the users
/// of your application. It usually is a convenient to start with a correctly
/// formatted file with all possible options inside.
///
/// # Example
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use std::path::PathBuf;
/// use confique::{Config, json5::FormatOptions};
///
/// /// App configuration.
/// #[derive(Config)]
/// struct Conf {
///     /// The color of the app.
///     color: String,
///
///     #[config(nested)]
///     log: LogConfig,
/// }
///
/// #[derive(Config)]
/// struct LogConfig {
///     /// If set to `true`, the app will log to stdout.
///     #[config(default = true)]
///     stdout: bool,
///
///     /// If this is set, the app will write logs to the given file. Of course,
///     /// the app has to have write access to that file.
///     #[config(env = "LOG_FILE")]
///     file: Option<PathBuf>,
/// }
///
/// const EXPECTED: &str = "\
/// // App configuration.
/// {
///   // The color of the app.
///   //
///   // Required! This value must be specified.
///   //color: ,
///
///   log: {
///     // If set to `true`, the app will log to stdout.
///     //
///     // Default value: true
///     //stdout: true,
///
///     // If this is set, the app will write logs to the given file. Of course,
///     // the app has to have write access to that file.
///     //
///     // Can also be specified via environment variable `LOG_FILE`.
///     //file: ,
///   },
/// }
/// ";
///
/// fn main() {
///     let json5 = confique::json5::template::<Conf>(FormatOptions::default());
///     assert_eq!(json5, EXPECTED);
/// }
/// ```
pub fn template<C: Config>(options: FormatOptions) -> String {
    let mut out = Json5Formatter::new(&options);
    format::template(&C::META, &mut out, &options.general, format_expr);
    out.finish()
}

/// Format an Expr as JSON5.
fn format_expr(expr: &Expr) -> String {
    PrintExpr(expr).to_string()
}

struct Json5Formatter {
    indent: u8,
    buffer: String,
    depth: u8,
}

impl Json5Formatter {
    fn new(options: &FormatOptions) -> Self {
        Self {
            indent: options.indent,
            buffer: String::new(),
            depth: 0,
        }
    }

    fn emit_indentation(&mut self) {
        let num_spaces = self.depth as usize * self.indent as usize;
        write!(self.buffer, "{: <1$}", "", num_spaces).unwrap();
    }

    fn dec_depth(&mut self) {
        self.depth = self
            .depth
            .checked_sub(1)
            .expect("formatter bug: ended too many nested");
    }
}

impl Json5Formatter {
    fn new_with_indent(indent: u8) -> Self {
        Self {
            indent,
            buffer: String::new(),
            depth: 0,
        }
    }
}

impl ConfigFormatter for Json5Formatter {
    fn buffer(&mut self) -> &mut String {
        &mut self.buffer
    }

    fn finish(self) -> String {
        assert_eq!(self.depth, 0, "formatter bug: lingering nested objects");
        self.buffer
    }

    fn comment(&mut self, comment: impl fmt::Display) {
        self.emit_indentation();
        writeln!(self.buffer, "//{comment}").unwrap();
    }

    fn field(&mut self, name: &'static str, value: &str) {
        self.emit_indentation();
        writeln!(self.buffer, "{name}: {value},").unwrap();
    }

    fn disabled_field(&mut self, name: &'static str, value: Option<&str>) {
        match value {
            Some(v) => self.comment(format_args!("{name}: {v},")),
            None => self.comment(format_args!("{name}: ,")),
        }
    }

    fn start_nested(&mut self, name: &'static str, doc: &[&'static str]) {
        doc.iter().for_each(|doc| self.comment(doc));
        self.emit_indentation();
        writeln!(self.buffer, "{name}: {{").unwrap();
        self.depth += 1;
    }

    fn end_nested(&mut self) {
        self.dec_depth();
        self.emit_indentation();
        self.buffer.push_str("},\n");
    }

    fn start_main(&mut self) {
        self.buffer.push_str("{\n");
        self.depth += 1;
    }

    fn end_main(&mut self) {
        self.dec_depth();
        self.buffer.push_str("}\n");
    }
}

/// Helper to emit `meta::Expr` into JSON5.
struct PrintExpr<'a>(&'a Expr);

impl From<&'static Expr> for PrintExpr<'static> {
    fn from(expr: &'static Expr) -> Self {
        Self(expr)
    }
}

impl fmt::Display for PrintExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        json5::to_string(&self.0)
            .expect("string serialization to JSON5 failed")
            .fmt(f)
    }
}

// ============================================================================
// Serialization (Config instance -> JSON5 string with comments)
// ============================================================================

#[cfg(all(feature = "serialize", feature = "serde_json"))]
mod serialization {
    use super::*;

    /// Options for serializing a configuration to JSON5.
    #[non_exhaustive]
    pub struct SerializeFormatOptions {
        /// Indentation per level. Default: 2.
        pub indent: u8,

        /// Non JSON5-specific options.
        pub general: format::FormatOptions,
    }

    impl Default for SerializeFormatOptions {
        fn default() -> Self {
            Self {
                indent: 2,
                general: Default::default(),
            }
        }
    }

    /// Serializes an instantiated configuration to a JSON5 string with comments.
    ///
    /// This can be used to save a configuration back to disk, including all the
    /// documentation comments from the original struct definition.
    ///
    /// The config type must implement both [`Config`] and [`serde::Serialize`].
    ///
    /// # Example
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use std::path::PathBuf;
    /// use confique::{Config, json5::SerializeFormatOptions};
    /// use serde::Serialize;
    ///
    /// /// App configuration.
    /// #[derive(Config, Serialize)]
    /// struct Conf {
    ///     /// The color of the app.
    ///     color: String,
    ///
    ///     #[config(nested)]
    ///     log: LogConfig,
    /// }
    ///
    /// #[derive(Config, Serialize)]
    /// struct LogConfig {
    ///     /// If set to `true`, the app will log to stdout.
    ///     #[config(default = true)]
    ///     stdout: bool,
    ///
    ///     /// If this is set, the app will write logs to the given file.
    ///     #[config(env = "LOG_FILE")]
    ///     file: Option<PathBuf>,
    /// }
    ///
    /// fn main() {
    ///     let config = Conf {
    ///         color: "blue".to_string(),
    ///         log: LogConfig {
    ///             stdout: false,
    ///             file: Some(PathBuf::from("/var/log/app.log")),
    ///         },
    ///     };
    ///
    ///     let json5 = confique::json5::serialize(&config, SerializeFormatOptions::default()).unwrap();
    ///     assert!(json5.contains("color: \"blue\""));
    ///     assert!(json5.contains("stdout: false"));
    /// }
    /// ```
    pub fn serialize<C: Config + Serialize>(
        config: &C,
        options: SerializeFormatOptions,
    ) -> Result<String, crate::Error> {
        // Serialize the config to a serde_json::Value (JSON5 is a superset of JSON)
        let value = serde_json::to_value(config).map_err(crate::Error::serialization)?;

        let mut out = Json5Formatter::new_with_indent(options.indent);
        format::serialize(&C::META, &value, &mut out, &options.general, format_expr);
        Ok(out.finish())
    }

    impl ValueAccess for serde_json::Value {
        fn get_field(&self, name: &str) -> Option<&Self> {
            self.as_object().and_then(|o| o.get(name))
        }

        fn is_null(&self) -> bool {
            self.is_null()
        }

        fn format_value(&self) -> String {
            match self {
                serde_json::Value::Null => "null".to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => format!("{:?}", s),
                serde_json::Value::Array(arr) => {
                    let items: Vec<_> = arr.iter().map(|v| v.format_value()).collect();
                    format!("[{}]", items.join(", "))
                }
                serde_json::Value::Object(obj) => {
                    let items: Vec<_> = obj
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v.format_value()))
                        .collect();
                    format!("{{ {} }}", items.join(", "))
                }
            }
        }
    }
}

#[cfg(all(feature = "serialize", feature = "serde_json"))]
pub use serialization::{serialize, SerializeFormatOptions};

#[cfg(test)]
mod tests {
    use super::{template, FormatOptions};
    use crate::test_utils::{self, include_format_output};
    use pretty_assertions::assert_str_eq;

    #[test]
    fn default() {
        let out = template::<test_utils::example1::Conf>(FormatOptions::default());
        assert_str_eq!(&out, include_format_output!("1-default.json5"));
    }

    #[test]
    fn no_comments() {
        let mut options = FormatOptions::default();
        options.general.comments = false;
        let out = template::<test_utils::example1::Conf>(options);
        assert_str_eq!(&out, include_format_output!("1-no-comments.json5"));
    }

    #[test]
    fn immediately_nested() {
        let out = template::<test_utils::example2::Conf>(Default::default());
        assert_str_eq!(&out, include_format_output!("2-default.json5"));
    }
}

#[cfg(all(test, feature = "serialize", feature = "serde_json"))]
mod serialize_tests {
    use pretty_assertions::assert_str_eq;

    use super::{serialize, template, FormatOptions, SerializeFormatOptions};
    use crate::{test_utils::example3::Conf, Config};

    #[test]
    fn round_trip_default_config() {
        let original = Conf::with_defaults();

        let mut options = SerializeFormatOptions::default();
        options.general.comments = false;
        let serialized = serialize(&original, options).unwrap();

        let parsed: Conf = Conf::builder()
            .preloaded(json5::from_str(&serialized).unwrap())
            .load()
            .unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn round_trip_custom_config() {
        let original = Conf {
            name: "custom-app".to_string(),
            server: crate::test_utils::example3::Server {
                port: 3000,
                host: "0.0.0.0".to_string(),
                tls: true,
            },
            log: crate::test_utils::example3::LogConfig {
                stdout: false,
                file: Some("/var/log/app.log".into()),
            },
        };

        let mut options = SerializeFormatOptions::default();
        options.general.comments = false;
        let serialized = serialize(&original, options).unwrap();

        let parsed: Conf = Conf::builder()
            .preloaded(json5::from_str(&serialized).unwrap())
            .load()
            .unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn serialize_default_equals_template() {
        let template_output = template::<Conf>(FormatOptions::default());
        let config = Conf::with_defaults();
        let serialized = serialize(&config, SerializeFormatOptions::default()).unwrap();

        assert_str_eq!(template_output, serialized);
    }
}
