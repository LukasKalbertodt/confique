//! TOML specific features. This module only exists if the Cargo feature `toml`
//! is enabled.

use std::fmt::{self, Write};

use crate::{
    format::{self, ConfigFormatter},
    meta::{Expr, MapKey},
    Config,
};

#[cfg(feature = "serialize")]
use serde::Serialize;

#[cfg(feature = "serialize")]
use crate::format::ValueAccess;

/// Options for generating a TOML template.
#[non_exhaustive]
#[derive(Default)]
pub struct FormatOptions {
    /// Indentation for nested tables. Default: 0.
    pub indent: u8,

    /// Non TOML-specific options.
    pub general: format::FormatOptions,
}

/// Formats the configuration description as a TOML file.
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
/// use confique::{Config, toml::FormatOptions};
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
/// ##color =
///
/// [log]
/// ## If set to `true`, the app will log to stdout.
/// ##
/// ## Default value: true
/// ##stdout = true
///
/// ## If this is set, the app will write logs to the given file. Of course,
/// ## the app has to have write access to that file.
/// ##
/// ## Can also be specified via environment variable `LOG_FILE`.
/// ##file =
/// ";
///
/// fn main() {
///     let toml = confique::toml::template::<Conf>(FormatOptions::default());
///     assert_eq!(toml, EXPECTED);
/// }
/// ```
pub fn template<C: Config>(options: FormatOptions) -> String {
    let mut out = TomlFormatter::new(&options);
    format::template(&C::META, &mut out, &options.general, format_expr);
    out.finish()
}

/// Format an Expr as TOML.
fn format_expr(expr: &Expr) -> String {
    PrintExpr(expr).to_string()
}

struct TomlFormatter {
    indent: u8,
    buffer: String,
    stack: Vec<String>,
}

impl TomlFormatter {
    fn new(options: &FormatOptions) -> Self {
        Self::new_with_indent(options.indent)
    }

    fn new_with_indent(indent: u8) -> Self {
        Self {
            indent,
            buffer: String::new(),
            stack: Vec::new(),
        }
    }

    fn emit_indentation(&mut self) {
        let num_spaces = self.stack.len() * self.indent as usize;
        write!(self.buffer, "{: <1$}", "", num_spaces).unwrap();
    }
}

impl ConfigFormatter for TomlFormatter {
    fn buffer(&mut self) -> &mut String {
        &mut self.buffer
    }

    fn finish(self) -> String {
        assert!(self.stack.is_empty(), "formatter bug: stack not empty");
        self.buffer
    }

    fn comment(&mut self, comment: impl fmt::Display) {
        self.emit_indentation();
        writeln!(self.buffer, "#{comment}").unwrap();
    }

    fn field(&mut self, name: &str, value: &str) {
        self.emit_indentation();
        writeln!(self.buffer, "{name} = {value}").unwrap();
    }

    fn disabled_field(&mut self, name: &str, value: Option<&str>) {
        match value {
            Some(v) => self.comment(format_args!("{name} = {v}")),
            None => self.comment(format_args!("{name} =")),
        }
    }

    fn start_nested(&mut self, name: &str, doc: &[&str]) {
        self.push_path(name);
        doc.iter().for_each(|doc| self.comment(doc));
        self.emit_indentation();
        writeln!(self.buffer, "[{}]", self.stack.join(".")).unwrap();
    }

    fn end_nested(&mut self) {
        self.pop_path();
    }

    fn start_array_element(&mut self, name: &str) {
        self.push_path(name);
        self.emit_indentation();
        writeln!(self.buffer, "[[{}]]", self.stack.join(".")).unwrap();
    }

    fn push_path(&mut self, name: &str) {
        self.stack.push(name.to_owned());
    }

    fn pop_path(&mut self) {
        self.stack.pop().expect("formatter bug: stack empty");
    }

    fn start_main(&mut self) {
        self.make_gap(1);
    }
}

/// Helper to emit `meta::Expr` into TOML.
struct PrintExpr<'a>(&'a Expr);

impl From<&'static Expr> for PrintExpr<'static> {
    fn from(expr: &'static Expr) -> Self {
        Self(expr)
    }
}

impl fmt::Display for PrintExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Expr::Map(entries) => {
                // TODO: pretty printing of long arrays onto multiple lines?
                f.write_str("{ ")?;
                for (i, entry) in entries.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }

                    match entry.key {
                        MapKey::Str(s) if is_valid_bare_key(s) => f.write_str(s)?,
                        _ => PrintExpr(&entry.key.into()).fmt(f)?,
                    }
                    f.write_str(" = ")?;
                    PrintExpr(&entry.value).fmt(f)?;
                }
                f.write_str(" }")?;
                Ok(())
            }

            // We special case floats as the TOML serializer below doesn't work
            // well with floats, not rounding them appropriately. See:
            // https://github.com/toml-rs/toml/issues/494
            //
            // For all non-NAN floats, the `Display` output is compatible with
            // TOML.
            Expr::Float(fv) if !fv.is_nan() => fv.fmt(f),

            // All these other types can simply be serialized as is.
            Expr::Str(_) | Expr::Float(_) | Expr::Integer(_) | Expr::Bool(_) | Expr::Array(_) => {
                let mut s = String::new();
                serde::Serialize::serialize(&self.0, toml::ser::ValueSerializer::new(&mut s))
                    .expect("string serialization to TOML failed");
                s.fmt(f)
            }
        }
    }
}

fn is_valid_bare_key(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

// ============================================================================
// Serialization (Config instance -> TOML string with comments)
// ============================================================================

#[cfg(feature = "serialize")]
mod serialization {
    use super::*;

    /// Options for serializing a configuration to TOML.
    #[non_exhaustive]
    #[derive(Default)]
    pub struct SerializeFormatOptions {
        /// Indentation for nested tables. Default: 0.
        pub indent: u8,

        /// Non TOML-specific options.
        pub general: format::FormatOptions,
    }

    /// Serializes an instantiated configuration to a TOML string with comments.
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
    /// use confique::{Config, toml::SerializeFormatOptions};
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
    ///     let toml = confique::toml::serialize(&config, SerializeFormatOptions::default()).unwrap();
    ///     assert!(toml.contains("color = \"blue\""));
    ///     assert!(toml.contains("stdout = false"));
    /// }
    /// ```
    pub fn serialize<C: Config + Serialize>(
        config: &C,
        options: SerializeFormatOptions,
    ) -> Result<String, crate::Error> {
        // Serialize the config to a toml::Value
        let value = toml::Value::try_from(config).map_err(crate::Error::serialization)?;

        let mut out = TomlFormatter::new_with_indent(options.indent);
        format::serialize(&C::META, &value, &mut out, &options.general, format_expr);
        Ok(out.finish())
    }

    impl ValueAccess for toml::Value {
        fn get_field(&self, name: &str) -> Option<&Self> {
            self.as_table().and_then(|t| t.get(name))
        }

        fn format_value(&self) -> String {
            // Use toml's own serialization for values
            match self {
                toml::Value::String(s) => format!("{:?}", s),
                toml::Value::Integer(i) => i.to_string(),
                toml::Value::Float(f) => {
                    // Ensure floats always have a decimal point
                    let s = f.to_string();
                    if s.contains('.') || s.contains('e') || s.contains('E') {
                        s
                    } else {
                        format!("{}.0", s)
                    }
                }
                toml::Value::Boolean(b) => b.to_string(),
                toml::Value::Datetime(dt) => dt.to_string(),
                toml::Value::Array(arr) => {
                    let items: Vec<_> = arr.iter().map(|v| v.format_value()).collect();
                    format!("[{}]", items.join(", "))
                }
                toml::Value::Table(table) => {
                    let items: Vec<_> = table
                        .iter()
                        .map(|(k, v)| {
                            let key = if is_valid_bare_key(k) {
                                k.clone()
                            } else {
                                format!("{:?}", k)
                            };
                            format!("{} = {}", key, v.format_value())
                        })
                        .collect();
                    format!("{{ {} }}", items.join(", "))
                }
            }
        }

        fn table_entries(&self) -> Option<Vec<(&str, &Self)>> {
            self.as_table()
                .map(|t| t.iter().map(|(k, v)| (k.as_str(), v)).collect())
        }

        fn nested_table_entries(&self) -> Option<Vec<(&str, &Self)>> {
            match self {
                toml::Value::Table(t) if !t.is_empty() && t.values().all(|v| v.is_table()) => {
                    Some(t.iter().map(|(k, v)| (k.as_str(), v)).collect())
                }
                _ => None,
            }
        }

        fn nested_array_entries(&self) -> Option<Vec<&Self>> {
            match self {
                toml::Value::Array(arr) if !arr.is_empty() && arr.iter().all(|v| v.is_table()) => {
                    Some(arr.iter().collect())
                }
                _ => None,
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
    use crate::format::ConfigFormatter;
    use crate::test_utils::{self, include_format_output};

    #[test]
    fn default() {
        let out = template::<test_utils::example1::Conf>(FormatOptions::default());
        assert_str_eq!(&out, include_format_output!("1-default.toml"));
    }

    #[test]
    fn no_comments() {
        let mut options = FormatOptions::default();
        options.general.comments = false;
        let out = template::<test_utils::example1::Conf>(options);
        assert_str_eq!(&out, include_format_output!("1-no-comments.toml"));
    }

    #[test]
    fn indent_2() {
        let mut options = FormatOptions::default();
        options.indent = 2;
        let out = template::<test_utils::example1::Conf>(options);
        assert_str_eq!(&out, include_format_output!("1-indent-2.toml"));
    }

    #[test]
    fn nested_gap_2() {
        let mut options = FormatOptions::default();
        options.general.nested_field_gap = 2;
        let out = template::<test_utils::example1::Conf>(options);
        assert_str_eq!(&out, include_format_output!("1-nested-gap-2.toml"));
    }

    #[test]
    fn immediately_nested() {
        let out = template::<test_utils::example2::Conf>(Default::default());
        assert_str_eq!(&out, include_format_output!("2-default.toml"));
    }

    #[test]
    fn skip_field_hidden_from_template() {
        use crate::meta::*;

        let meta = Meta {
            name: "Conf",
            doc: &[],
            fields: &[
                Field {
                    name: "port",
                    doc: &[" Visible field."],
                    kind: FieldKind::Leaf {
                        env: None,
                        kind: LeafKind::Required {
                            default: Some(Expr::Integer(Integer::U16(8080))),
                        },
                    },
                    skip: false,
                },
                Field {
                    name: "secret",
                    doc: &[" This should be hidden."],
                    kind: FieldKind::Leaf {
                        env: None,
                        kind: LeafKind::Optional,
                    },
                    skip: true,
                },
                Field {
                    name: "name",
                    doc: &[" Also visible."],
                    kind: FieldKind::Leaf {
                        env: None,
                        kind: LeafKind::Required { default: None },
                    },
                    skip: false,
                },
            ],
        };

        let mut out = super::TomlFormatter::new(&FormatOptions::default());
        crate::format::template(&meta, &mut out, &FormatOptions::default().general, super::format_expr);
        let result = out.finish();

        assert!(result.contains("port"), "visible field 'port' should appear in template");
        assert!(result.contains("name"), "visible field 'name' should appear in template");
        assert!(!result.contains("secret"), "skipped field 'secret' should NOT appear in template");
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

        // Serialize to TOML
        let mut options = SerializeFormatOptions::default();
        options.general.comments = false; // No comments for clean parsing
        let serialized = serialize(&original, options).unwrap();

        // Deserialize back
        let parsed: Conf = Conf::builder()
            .preloaded(toml::from_str(&serialized).unwrap())
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
            .preloaded(toml::from_str(&serialized).unwrap())
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

    #[test]
    fn table_of_tables_renders_as_sections() {
        use std::collections::BTreeMap;
        use crate::test_utils::example4;

        let config = example4::Conf {
            name: "my-cluster".to_string(),
            hosts: BTreeMap::from([
                ("node1".to_string(), example4::Host {
                    root_img: "/path/to/server.qcow2".to_string(),
                    num_drives: Some(4),
                }),
                ("node2".to_string(), example4::Host {
                    root_img: "/path/to/client.qcow2".to_string(),
                    num_drives: None,
                }),
            ]),
        };

        let mut options = SerializeFormatOptions::default();
        options.general.comments = false;
        let serialized = serialize(&config, options).unwrap();

        // Should use TOML sections, not inline tables
        assert!(
            serialized.contains("[hosts.node1]"),
            "expected [hosts.node1] section header, got:\n{serialized}"
        );
        assert!(
            serialized.contains("[hosts.node2]"),
            "expected [hosts.node2] section header, got:\n{serialized}"
        );
        assert!(
            !serialized.contains("hosts = {"),
            "should NOT render as inline table, got:\n{serialized}"
        );
    }

    #[test]
    fn array_of_tables_renders_as_double_bracket_sections() {
        use crate::test_utils::example5;

        let config = example5::Conf {
            name: "my-cluster".to_string(),
            post_init: vec![
                example5::Hook {
                    kind: "exec".to_string(),
                    exec: Some("/bin/setup".to_string()),
                },
                example5::Hook {
                    kind: "wait".to_string(),
                    exec: None,
                },
            ],
        };

        let mut options = SerializeFormatOptions::default();
        options.general.comments = false;
        let serialized = serialize(&config, options).unwrap();

        let header_count = serialized.matches("[[post_init]]").count();
        assert_eq!(
            header_count, 2,
            "expected 2 [[post_init]] headers, got:\n{serialized}"
        );
        assert!(
            !serialized.contains("post_init = ["),
            "should NOT render as inline array, got:\n{serialized}"
        );
    }

    #[test]
    fn array_of_tables_round_trip() {
        use crate::test_utils::example5;

        let original = example5::Conf {
            name: "my-cluster".to_string(),
            post_init: vec![
                example5::Hook {
                    kind: "exec".to_string(),
                    exec: Some("/bin/setup".to_string()),
                },
                example5::Hook {
                    kind: "wait".to_string(),
                    exec: None,
                },
            ],
        };

        let mut options = SerializeFormatOptions::default();
        options.general.comments = false;
        let serialized = serialize(&original, options).unwrap();

        let parsed: example5::Conf = example5::Conf::builder()
            .preloaded(toml::from_str(&serialized).unwrap())
            .load()
            .unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn empty_array_of_tables_renders_inline() {
        use crate::test_utils::example5;

        let config = example5::Conf {
            name: "my-cluster".to_string(),
            post_init: vec![],
        };

        let mut options = SerializeFormatOptions::default();
        options.general.comments = false;
        let serialized = serialize(&config, options).unwrap();

        assert!(
            serialized.contains("post_init = []"),
            "empty Vec should render as inline empty array, got:\n{serialized}"
        );
        assert!(
            !serialized.contains("[[post_init]]"),
            "empty Vec should NOT produce array-of-tables headers, got:\n{serialized}"
        );
    }

    #[test]
    fn table_of_tables_round_trip() {
        use std::collections::BTreeMap;
        use crate::test_utils::example4;

        let original = example4::Conf {
            name: "test-cluster".to_string(),
            hosts: BTreeMap::from([
                ("server".to_string(), example4::Host {
                    root_img: "/img/server.qcow2".to_string(),
                    num_drives: Some(8),
                }),
                ("client".to_string(), example4::Host {
                    root_img: "/img/client.qcow2".to_string(),
                    num_drives: None,
                }),
            ]),
        };

        let mut options = SerializeFormatOptions::default();
        options.general.comments = false;
        let serialized = serialize(&original, options).unwrap();

        // Parse back via TOML and confique builder
        let parsed: example4::Conf = example4::Conf::builder()
            .preloaded(toml::from_str(&serialized).unwrap())
            .load()
            .unwrap();

        assert_eq!(original, parsed);
    }
}
