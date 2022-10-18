//! TOML specific features. This module only exists if the Cargo feature `toml`
//! is enabled.

use std::fmt::{self, Write};

use crate::{
    Config,
    format::{DefaultValueComment, add_empty_line, assert_single_trailing_newline},
    meta::{Expr, FieldKind, LeafKind, Meta},
};



/// Options for generating a TOML template.
pub struct FormatOptions {
    // TODO: think about forward/backwards compatibility.

    /// Indentation for nested tables. Default: 0.
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
            indent: 0,
            comments: true,
            env_keys: true,
        }
    }
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
///     let toml = confique::toml::format::<Conf>(FormatOptions::default());
///     assert_eq!(toml, EXPECTED);
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
    format_impl(&mut out, meta, vec![], &options);
    assert_single_trailing_newline(&mut out);

    out
}

fn format_impl(
    s: &mut String,
    meta: &Meta,
    path: Vec<&str>,
    options: &FormatOptions,
) {
    /// Like `println!` but into `s` and with indentation.
    macro_rules! emit {
        ($fmt:literal $(, $args:expr)* $(,)?) => {{
            // Writing to a string never fails, we can unwrap.
            let indent = path.len().saturating_sub(1) * options.indent as usize;
            write!(s, "{: <1$}", "", indent).unwrap();
            writeln!(s, $fmt $(, $args)*).unwrap();
        }};
    }

    // Output all leaf fields first
    let leaf_fields = meta.fields.iter().filter_map(|f| match &f.kind {
        FieldKind::Leaf { kind, env } => Some((f, kind, env)),
        _ => None,
    });
    for (field, kind, env) in leaf_fields {
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

            if let Some(env) = env {
                empty_sep_doc_line!();
                emit!("# Can also be specified via environment variable `{env}`.")
            }
        }

        if let LeafKind::Required { default } = kind {
            // Emit comment about default value or the value being required
            if options.comments {
                empty_sep_doc_line!();
                emit!("# {}", DefaultValueComment(default.as_ref().map(PrintExpr)));
            }

            // Emit the actual line with the name and optional value
            match default {
                Some(v) => emit!("#{} = {}", field.name, PrintExpr(v)),
                None => emit!("#{} =", field.name),
            }
        } else {
            emit!("#{} =", field.name);
        }

        if options.comments {
            add_empty_line(s);
        }
    }

    // Then all nested fields recursively
    let nested_fields = meta.fields.iter().filter_map(|f| match &f.kind {
        FieldKind::Nested { meta } => Some((f, meta)),
        _ => None,
    });
    for (field, meta) in nested_fields {
        emit!("");
        // add_empty_line(s);
        if options.comments {
            field.doc.iter().for_each(|doc| emit!("#{doc}"));
        }

        let child_path = path.iter().copied().chain([field.name]).collect::<Vec<_>>();
        emit!("[{}]", child_path.join("."));
        format_impl(s, meta, child_path, options);

        if options.comments {
            add_empty_line(s);
        }
    }
}

/// Helper to emit `meta::Expr` into TOML.
struct PrintExpr(&'static Expr);

impl fmt::Display for PrintExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.0 {
            Expr::Str(v) => toml::Value::String(v.to_owned()).fmt(f),
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
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{self, include_format_output};
    use super::{format, FormatOptions};
    use pretty_assertions::assert_str_eq;

    #[test]
    fn default() {
        let out = format::<test_utils::example1::Conf>(FormatOptions::default());
        assert_str_eq!(&out, include_format_output!("1-default.toml"));
    }

    #[test]
    fn no_comments() {
        let out = format::<test_utils::example1::Conf>(FormatOptions {
            comments: false,
            indent: 0,
            .. FormatOptions::default()
        });
        assert_str_eq!(&out, include_format_output!("1-no-comments.toml"));
    }
}
