//! TOML specific features. This module only exists if the Cargo feature `toml`
//! is enabled.

use std::fmt::{self, Write};

use crate::{Config, meta::{Expr, FieldKind, Meta}};



/// Options for generating a TOML template.
pub struct FormatOptions {
    // TODO: think about forward/backwards compatibility.

    /// Indentation if nested tables. Default: 0.
    pub indent: u8,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 0,
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
///
/// let toml = confique::toml::format::<Conf>(FormatOptions::default());
/// assert_eq!(toml, "\
///     ## App configuration.\n\
///     \n\
///     ## The color of the app.\n\
///     ##\n\
///     ## Required! This value must be specified.\n\
///     ##color =\n\
///     \n\
///     [log]\n\
///     ## If set to `true`, the app will log to stdout.\n\
///     ##\n\
///     ## Default value: true\n\
///     ##stdout = true\n\
///     \n\
///     ## If this is set, the app will write logs to the given file. Of course,\n\
///     ## the app has to have write access to that file.\n\
///     ##file =\n\
/// ");
/// ```
pub fn format<C: Config>(options: FormatOptions) -> String {
    let mut out = String::new();
    let meta = &C::META;

    // Print root docs.
    for doc in meta.doc {
        writeln!(out, "#{}", doc).unwrap();
    }
    if !meta.doc.is_empty() {
        add_empty_line(&mut out);
    }

    // Recursively format all nested objects and fields
    format_impl(&mut out, meta, vec![], &options);

    // Make sure there is only a single trailing newline.
    while out.ends_with("\n\n") {
        out.pop();
    }

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

    if !path.is_empty() {
        emit!("[{}]", path.join("."));
    }

    for field in meta.fields {
        for doc in field.doc {
            emit!("#{}", doc);
        }

        match &field.kind {
            FieldKind::Leaf { default } => {
                // Emit comment about default value or the value being required
                match default {
                    Some(v) => {
                        if !field.doc.is_empty() {
                            emit!("#");
                        }
                        emit!("# Default value: {}", PrintExpr(v));
                    }
                    None => {
                        if !field.optional {
                            if !field.doc.is_empty() {
                                emit!("#");
                            }
                            emit!("# Required! This value must be specified.");
                        }
                    }
                }

                // Emit the actual line with the name and optional value
                match default {
                    Some(v) => emit!("#{} = {}", field.name, PrintExpr(v)),
                    None => emit!("#{} =", field.name),
                }
            }

            FieldKind::Nested { meta } => {
                let child_path = path.iter().copied().chain([field.name]).collect();
                format_impl(s, meta, child_path, options);
            }
        }

        add_empty_line(s);
    }
}

/// Adds zero, one or two line breaks to make sure that there are at least two
/// line breaks at the end of the string.
fn add_empty_line(out: &mut String) {
    match () {
        () if out.ends_with("\n\n") => {},
        () if out.ends_with('\n') => out.push('\n'),
        _ => out.push_str("\n\n"),
    }
}

/// Helper to emit `meta::Expr` into TOML.
struct PrintExpr(&'static Expr);

impl fmt::Display for PrintExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Expr::Str(v) => write!(f, "\"{}\"", v),
            Expr::Float(v) => v.fmt(f),
            Expr::Integer(v) => v.fmt(f),
            Expr::Bool(v) => v.fmt(f),
        }
    }
}
