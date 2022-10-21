//! JSON5 specific features. This module only exists if the Cargo feature
//! `json5` is enabled.

use std::fmt::{self, Write};

use crate::{
    Config,
    template::{self, Formatter},
    meta::Expr,
};



/// Options for generating a JSON5 template.
#[non_exhaustive]
pub struct FormatOptions {
    /// Indentation per level. Default: 2.
    pub indent: u8,

    /// Non JSON5-specific options.
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
    template::format::<C>(&mut out, options.general);
    out.finish()
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
        self.depth = self.depth.checked_sub(1).expect("formatter bug: ended too many nested");
    }
}

impl Formatter for Json5Formatter {
    type ExprPrinter = PrintExpr;

    fn buffer(&mut self) -> &mut String {
        &mut self.buffer
    }

    fn comment(&mut self, comment: impl fmt::Display) {
        self.emit_indentation();
        writeln!(self.buffer, "//{comment}").unwrap();
    }

    fn disabled_field(&mut self, name: &str, value: Option<&'static Expr>) {
        match value.map(PrintExpr) {
            None => self.comment(format_args!("{name}: ,")),
            Some(v) => self.comment(format_args!("{name}: {v},")),
        };
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

    fn finish(self) -> String {
        assert_eq!(self.depth, 0, "formatter bug: lingering nested objects");
        self.buffer
    }
}

/// Helper to emit `meta::Expr` into JSON5.
struct PrintExpr(&'static Expr);

impl From<&'static Expr> for PrintExpr {
    fn from(expr: &'static Expr) -> Self {
        Self(expr)
    }
}

impl fmt::Display for PrintExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        json5::to_string(&self.0)
            .expect("string serialization to JSON5 failed")
            .fmt(f)
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
