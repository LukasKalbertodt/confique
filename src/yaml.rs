//! YAML specific features. This module only exists if the Cargo feature `yaml`
//! is enabled.

use std::fmt::{self, Write};

use crate::{
    Config,
    format::{DefaultValueComment, add_empty_line, assert_single_trailing_newline},
    meta::{Expr, FieldKind, LeafKind, Meta},
};



/// Options for generating a YAML template.
pub struct FormatOptions {
    // TODO: think about forward/backwards compatibility.

    /// Amount of indentation in spaces. Default: 2.
    pub indent: u8,

    /// Whether to include doc comments (with your own text and information
    /// about whether a value is required and/or has a default). Default:
    /// true.
    pub comments: bool,

    /// If `comments` and this field are `true`, leaf fields with `env = "FOO"`
    /// attribute will have a line like this added:
    ///
    /// ```text
    /// # Can also be specified via environment variable `FOO`.
    /// ```
    ///
    /// Default: `true`.
    pub env_keys: bool,

    // Potential future options:
    // - Comment out default values (`#foo = 3` vs `foo = 3`)
    // - Which docs to include from nested objects
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            comments: true,
            env_keys: true,
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
///     let yaml = confique::yaml::format::<Conf>(FormatOptions::default());
///     assert_eq!(yaml, EXPECTED);
/// }
/// ```
pub fn format<C: Config>(options: FormatOptions) -> String {
    let mut out = String::new();
    let meta = &C::META;

    // Print root docs.
    if options.comments {
        meta.doc.iter().for_each(|doc| writeln!(out, "#{doc}").unwrap());
        if !meta.doc.is_empty() {
            add_empty_line(&mut out);
        }
    }

    // Recursively format all nested objects and fields
    format_impl(&mut out, meta, 0, &options);
    assert_single_trailing_newline(&mut out);

    out
}

fn format_impl(
    s: &mut String,
    meta: &Meta,
    depth: usize,
    options: &FormatOptions,
) {
    /// Like `println!` but into `s` and with indentation.
    macro_rules! emit {
        ($fmt:literal $(, $args:expr)* $(,)?) => {{
            // Writing to a string never fails, we can unwrap.
            let indent = depth * options.indent as usize;
            write!(s, "{: <1$}", "", indent).unwrap();
            writeln!(s, $fmt $(, $args)*).unwrap();
        }};
    }

    for field in meta.fields {
        let mut emitted_something = false;
        macro_rules! empty_sep_doc_line {
            () => {
                if emitted_something {
                    emit!("#");
                }
            };
        }

        if options.comments {
            field.doc.iter().for_each(|doc| emit!("#{doc}"));
            emitted_something = !field.doc.is_empty();

            if let FieldKind::Leaf { env: Some(env), .. } = field.kind {
                empty_sep_doc_line!();
                emit!("# Can also be specified via environment variable `{env}`.")
            }
        }

        match &field.kind {
            FieldKind::Leaf { kind: LeafKind::Required { default }, .. } => {
                // Emit comment about default value or the value being required
                if options.comments {
                    empty_sep_doc_line!();
                    emit!("# {}", DefaultValueComment(default.as_ref().map(PrintExpr)));
                }

                // Emit the actual line with the name and optional value
                match default {
                    Some(v) => emit!("#{}: {}", field.name, PrintExpr(v)),
                    None => emit!("#{}:", field.name),
                }
            }

            FieldKind::Leaf { kind: LeafKind::Optional, .. } => emit!("#{}:", field.name),

            FieldKind::Nested { meta } => {
                emit!("{}:", field.name);
                format_impl(s, meta, depth + 1, options);
            }
        }

        if options.comments {
            add_empty_line(s);
        }
    }
}

/// Helper to emit `meta::Expr` into YAML.
struct PrintExpr(&'static Expr);

impl fmt::Display for PrintExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.0 {
            Expr::Str(v) => {
                // This is a bit ugly. Sadly, no YAML crate in our dependency
                // tree has an API to serialize a string only, without emitting
                // the `---` at the start of the document. But instead of
                // implementing the quoting logic ourselves (which is really
                // complicated as it turns out!), we use this hack.
                let value = serde_yaml::Value::String(v.to_owned());
                let serialized = serde_yaml::to_string(&value).unwrap();
                serialized[4..].trim_end_matches('\n').fmt(f)
            },
            Expr::Float(v) => v.fmt(f),
            Expr::Integer(v) => v.fmt(f),
            Expr::Bool(v) => v.fmt(f),
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
        }
    }
}
