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

    // Potential future options:
    // - Comment out default values (`#foo = 3` vs `foo = 3`)
    // - Which docs to include from nested objects
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 0,
            comments: true,
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
///     file: Option<PathBuf>,
/// }
///
/// fn main() {
///     let toml = confique::toml::format::<Conf>(FormatOptions::default());
///     assert_eq!(toml, "\
///         ## App configuration.\n\
///         \n\
///         ## The color of the app.\n\
///         ##\n\
///         ## Required! This value must be specified.\n\
///         ##color =\n\
///         \n\
///         \n\
///         [log]\n\
///         ## If set to `true`, the app will log to stdout.\n\
///         ##\n\
///         ## Default value: true\n\
///         ##stdout = true\n\
///         \n\
///         ## If this is set, the app will write logs to the given file. Of course,\n\
///         ## the app has to have write access to that file.\n\
///         ##file =\n\
///     ");
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
    for (field, kind, _env) in leaf_fields {
        if options.comments {
            field.doc.iter().for_each(|doc| emit!("#{doc}"));
        }

        if let LeafKind::Required { default } = kind {
            // Emit comment about default value or the value being required
            if options.comments {
                if !field.doc.is_empty() {
                    emit!("#");
                }
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
        });
        assert_str_eq!(&out, include_format_output!("1-no-comments.toml"));
    }
}
