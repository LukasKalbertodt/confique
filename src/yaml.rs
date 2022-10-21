//! YAML specific features. This module only exists if the Cargo feature `yaml`
//! is enabled.

use std::fmt::{self, Write};

use crate::{
    Config,
    template::{self, Formatter},
    meta::Expr,
};



/// Options for generating a YAML template.
#[non_exhaustive]
pub struct FormatOptions {
    /// Amount of indentation in spaces. Default: 2.
    pub indent: u8,

    /// Non YAML-specific options.
    pub general: template::FormatOptions,
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
    template::format::<C>(&mut out, options.general);
    out.finish()
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

impl Formatter for YamlFormatter {
    type ExprPrinter = PrintExpr;

    fn buffer(&mut self) -> &mut String {
        &mut self.buffer
    }

    fn comment(&mut self, comment: impl fmt::Display) {
        self.emit_indentation();
        writeln!(self.buffer, "#{comment}").unwrap();
    }

    fn disabled_field(&mut self, name: &str, value: Option<&'static Expr>) {
        match value.map(PrintExpr) {
            None => self.comment(format_args!("{name}:")),
            Some(v) => self.comment(format_args!("{name}: {v}")),
        };
    }

    fn start_nested(&mut self, name: &'static str, doc: &[&'static str]) {
        doc.iter().for_each(|doc| self.comment(doc));
        self.emit_indentation();
        writeln!(self.buffer, "{name}:").unwrap();
        self.depth += 1;
    }

    fn end_nested(&mut self) {
        self.depth = self.depth.checked_sub(1).expect("formatter bug: ended too many nested");
    }

    fn start_main(&mut self) {
        self.make_gap(1);
    }

    fn finish(self) -> String {
        assert_eq!(self.depth, 0, "formatter bug: lingering nested objects");
        self.buffer
    }
}

/// Helper to emit `meta::Expr` into YAML.
struct PrintExpr(&'static Expr);

impl From<&'static Expr> for PrintExpr {
    fn from(expr: &'static Expr) -> Self {
        Self(expr)
    }
}

impl fmt::Display for PrintExpr {
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

            // All these other types can simply be serialized as is.
            Expr::Str(_) | Expr::Float(_) | Expr::Integer(_) | Expr::Bool(_) => {
                let out = serde_yaml::to_string(&self.0).expect("string serialization to YAML failed");

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


#[cfg(test)]
mod tests {
    use crate::test_utils::{self, include_format_output};
    use super::{template, FormatOptions};
    use pretty_assertions::assert_str_eq;

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
