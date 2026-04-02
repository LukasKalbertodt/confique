//! YAML specific features. This module only exists if the Cargo feature `yaml`
//! is enabled.

use std::fmt::{self, Write};

use crate::{
    format::{self, ConfigFormatter},
    meta::Expr,
    Config,
};

#[cfg(feature = "serialize")]
use serde::Serialize;

#[cfg(feature = "serialize")]
use crate::format::ValueAccess;

/// Options for generating a YAML template.
#[non_exhaustive]
pub struct FormatOptions {
    /// Amount of indentation in spaces. Default: 2.
    pub indent: u8,

    /// Non YAML-specific options.
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

/// Formats the configuration description as a YAML file.
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
/// use confique::{Config, yaml::FormatOptions};
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
/// ## App configuration.
///
/// ## The color of the app.
/// ##
/// ## Required! This value must be specified.
/// ##color:
///
/// log:
///   ## If set to `true`, the app will log to stdout.
///   ##
///   ## Default value: true
///   ##stdout: true
///
///   ## If this is set, the app will write logs to the given file. Of course,
///   ## the app has to have write access to that file.
///   ##
///   ## Can also be specified via environment variable `LOG_FILE`.
///   ##file:
/// ";
///
///
/// fn main() {
///     let yaml = confique::yaml::template::<Conf>(FormatOptions::default());
///     assert_eq!(yaml, EXPECTED);
/// }
/// ```
pub fn template<C: Config>(options: FormatOptions) -> String {
    let mut out = YamlFormatter::new(&options);
    format::template(&C::META, &mut out, &options.general, format_expr);
    out.finish()
}

/// Format an Expr as YAML.
fn format_expr(expr: &Expr) -> String {
    PrintExpr(expr).to_string()
}

struct YamlFormatter {
    indent: u8,
    buffer: String,
    depth: u8,
}

impl YamlFormatter {
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
}

impl YamlFormatter {
    fn new_with_indent(indent: u8) -> Self {
        Self {
            indent,
            buffer: String::new(),
            depth: 0,
        }
    }
}

impl ConfigFormatter for YamlFormatter {
    fn buffer(&mut self) -> &mut String {
        &mut self.buffer
    }

    fn finish(self) -> String {
        assert_eq!(self.depth, 0, "formatter bug: lingering nested objects");
        self.buffer
    }

    fn comment(&mut self, comment: impl fmt::Display) {
        self.emit_indentation();
        writeln!(self.buffer, "#{comment}").unwrap();
    }

    fn field(&mut self, name: &str, value: &str) {
        self.emit_indentation();
        writeln!(self.buffer, "{name}: {value}").unwrap();
    }

    fn disabled_field(&mut self, name: &str, value: Option<&str>) {
        match value {
            Some(v) => self.comment(format_args!("{name}: {v}")),
            None => self.comment(format_args!("{name}:")),
        }
    }

    fn start_nested(&mut self, name: &str, doc: &[&str]) {
        doc.iter().for_each(|doc| self.comment(doc));
        self.emit_indentation();
        writeln!(self.buffer, "{name}:").unwrap();
        self.depth += 1;
    }

    fn end_nested(&mut self) {
        self.depth = self
            .depth
            .checked_sub(1)
            .expect("formatter bug: ended too many nested");
    }

    fn start_main(&mut self) {
        self.make_gap(1);
    }
}

/// Helper to emit `meta::Expr` into YAML.
struct PrintExpr<'a>(&'a Expr);

impl From<&'static Expr> for PrintExpr<'static> {
    fn from(expr: &'static Expr) -> Self {
        Self(expr)
    }
}

impl fmt::Display for PrintExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.0 {
            // We have to special case arrays as the normal formatter only emits
            // multi line lists.
            Expr::Array(items) => {
                // TODO: pretty printing of long arrays onto multiple lines?
                f.write_char('[')?;
                for (i, item) in items.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    PrintExpr(item).fmt(f)?;
                }
                f.write_char(']')?;
                Ok(())
            }

            Expr::Map(entries) => {
                // TODO: pretty printing of long arrays onto multiple lines?
                f.write_str("{ ")?;
                for (i, entry) in entries.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    PrintExpr(&entry.key.into()).fmt(f)?;
                    f.write_str(": ")?;
                    PrintExpr(&entry.value).fmt(f)?;
                }
                f.write_str(" }")?;
                Ok(())
            }

            // All these other types can simply be serialized as is.
            Expr::Str(_) | Expr::Float(_) | Expr::Integer(_) | Expr::Bool(_) => {
                let out =
                    serde_yaml::to_string(&self.0).expect("string serialization to YAML failed");

                // Unfortunately, `serde_yaml` cannot serialize these values on its own
                // without embedding them in a full document (starting with `---` and
                // ending with a newline). So we need to cleanup.
                out.strip_prefix("---\n")
                    .unwrap_or(&out)
                    .trim_matches('\n')
                    .fmt(f)
            }
        }
    }
}

// ============================================================================
// Serialization (Config instance -> YAML string with comments)
// ============================================================================

#[cfg(feature = "serialize")]
mod serialization {
    use super::*;

    /// Options for serializing a configuration to YAML.
    #[non_exhaustive]
    pub struct SerializeFormatOptions {
        /// Amount of indentation in spaces. Default: 2.
        pub indent: u8,

        /// Non YAML-specific options.
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

    /// Serializes an instantiated configuration to a YAML string with comments.
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
    /// use confique::{Config, yaml::SerializeFormatOptions};
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
    ///     let yaml = confique::yaml::serialize(&config, SerializeFormatOptions::default()).unwrap();
    ///     assert!(yaml.contains("color: blue"));
    ///     assert!(yaml.contains("stdout: false"));
    /// }
    /// ```
    pub fn serialize<C: Config + Serialize>(
        config: &C,
        options: SerializeFormatOptions,
    ) -> Result<String, crate::Error> {
        // Serialize the config to a serde_yaml::Value
        let value = serde_yaml::to_value(config).map_err(crate::Error::serialization)?;

        let mut out = YamlFormatter::new_with_indent(options.indent);
        format::serialize(&C::META, &value, &mut out, &options.general, format_expr);
        Ok(out.finish())
    }

    impl ValueAccess for serde_yaml::Value {
        fn get_field(&self, name: &str) -> Option<&Self> {
            self.as_mapping().and_then(|m| m.get(name))
        }

        fn is_null(&self) -> bool {
            self.is_null()
        }

        fn format_value(&self) -> String {
            match self {
                serde_yaml::Value::Null => "null".to_string(),
                serde_yaml::Value::Bool(b) => b.to_string(),
                serde_yaml::Value::Number(n) => n.to_string(),
                serde_yaml::Value::String(s) => {
                    // Check if the string needs quoting
                    if s.is_empty()
                        || s.contains(':')
                        || s.contains('#')
                        || s.contains('\n')
                        || s.starts_with(' ')
                        || s.ends_with(' ')
                        || s == "true"
                        || s == "false"
                        || s == "null"
                        || s == "~"
                    {
                        format!("{:?}", s)
                    } else {
                        s.clone()
                    }
                }
                serde_yaml::Value::Sequence(seq) => {
                    let items: Vec<_> = seq.iter().map(|v| v.format_value()).collect();
                    format!("[{}]", items.join(", "))
                }
                serde_yaml::Value::Mapping(map) => {
                    let items: Vec<_> = map
                        .iter()
                        .filter_map(|(k, v)| {
                            let key = k.as_str()?;
                            Some(format!("{}: {}", key, v.format_value()))
                        })
                        .collect();
                    format!("{{ {} }}", items.join(", "))
                }
                serde_yaml::Value::Tagged(tagged) => {
                    // Handle tagged values by formatting the inner value
                    tagged.value.format_value()
                }
            }
        }
    }
}

#[cfg(feature = "serialize")]
pub use serialization::{serialize, SerializeFormatOptions};

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_str_eq;

    use super::{template, FormatOptions};
    use crate::test_utils::{self, include_format_output};

    #[test]
    fn default() {
        let out = template::<test_utils::example1::Conf>(FormatOptions::default());
        assert_str_eq!(&out, include_format_output!("1-default.yaml"));
    }

    #[test]
    fn no_comments() {
        let mut options = FormatOptions::default();
        options.general.comments = false;
        let out = template::<test_utils::example1::Conf>(options);
        assert_str_eq!(&out, include_format_output!("1-no-comments.yaml"));
    }

    #[test]
    fn immediately_nested() {
        let out = template::<test_utils::example2::Conf>(Default::default());
        assert_str_eq!(&out, include_format_output!("2-default.yaml"));
    }
}

#[cfg(all(test, feature = "serialize"))]
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
            .preloaded(serde_yaml::from_str(&serialized).unwrap())
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
            .preloaded(serde_yaml::from_str(&serialized).unwrap())
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
