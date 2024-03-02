//! TOML specific features. This module only exists if the Cargo feature `toml`
//! is enabled.

use std::fmt::{self, Write};

use crate::{
    meta::{Expr, MapKey},
    template::{self, Formatter},
    Config,
};



/// Options for generating a TOML template.
#[non_exhaustive]
pub struct FormatOptions {
    /// Indentation for nested tables. Default: 0.
    pub indent: u8,

    /// Non TOML-specific options.
    pub general: template::FormatOptions,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 0,
            general: Default::default(),
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
    template::format(&C::META, &mut out, options.general);
    out.finish()
}

struct TomlFormatter {
    indent: u8,
    buffer: String,
    stack: Vec<&'static str>,
}

impl TomlFormatter {
    fn new(options: &FormatOptions) -> Self {
        Self {
            indent: options.indent,
            buffer: String::new(),
            stack: Vec::new(),
        }
    }

    fn emit_indentation(&mut self) {
        let num_spaces = self.stack.len() * self.indent as usize;
        write!(self.buffer, "{: <1$}", "", num_spaces).unwrap();
    }
}

impl Formatter for TomlFormatter {
    type ExprPrinter = PrintExpr<'static>;

    fn buffer(&mut self) -> &mut String {
        &mut self.buffer
    }

    fn comment(&mut self, comment: impl fmt::Display) {
        self.emit_indentation();
        writeln!(self.buffer, "#{comment}").unwrap();
    }

    fn disabled_field(&mut self, name: &str, value: Option<&'static Expr>) {
        match value.map(PrintExpr) {
            None => self.comment(format_args!("{name} =")),
            Some(v) => self.comment(format_args!("{name} = {v}")),
        };
    }

    fn start_nested(&mut self, name: &'static str, doc: &[&'static str]) {
        self.stack.push(name);
        doc.iter().for_each(|doc| self.comment(doc));
        self.emit_indentation();
        writeln!(self.buffer, "[{}]", self.stack.join(".")).unwrap();
    }

    fn end_nested(&mut self) {
        self.stack.pop().expect("formatter bug: stack empty");
    }

    fn start_main(&mut self) {
        self.make_gap(1);
    }

    fn finish(self) -> String {
        assert!(self.stack.is_empty(), "formatter bug: stack not empty");
        self.buffer
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
            },

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
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_str_eq;

    use crate::test_utils::{self, include_format_output};
    use super::{template, FormatOptions};

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
}
