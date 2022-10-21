//! Utilities for creating a "configuration template".
//!
//! A config template is a description of all possible configuration values with
//! their default values and other information. This is super useful to give to
//! the users of your application as a starting point.

use std::fmt;

use crate::{Config, meta::{Meta, FieldKind, LeafKind, Expr}};


/// Trait abstracting over the format differences when it comes to formatting a
/// configuration template.
///
/// To implement this yourself, take a look at the existing impls for guidance.
pub(crate) trait Formatter {
    /// A type that is used to print expressions.
    type ExprPrinter: fmt::Display + From<&'static Expr>;

    /// Internal buffer, mainly used for `make_gap` and similar methods.
    fn buffer(&mut self) -> &mut String;

    /// Returns internal buffer by value.
    fn finish(self) -> String;

    /// Write a comment, e.g. `format!("#{comment}")`. Don't add a space after
    /// your comment token.
    fn comment(&mut self, comment: impl fmt::Display);

    /// Write a commented-out field with optional value, e.g. `format!("#{name} = {value}")`.
    fn disabled_field(&mut self, name: &'static str, value: Option<&'static Expr>);

    /// Start a nested configuration section with the given name.
    fn start_nested(&mut self, name: &'static str, doc: &[&'static str]);

    /// End a nested configuration section.
    fn end_nested(&mut self);

    /// Called after the global docs are written and before and fields are
    /// emitted. Default impl does nothing.
    fn start_main(&mut self) {}

    /// Called after all fields have been emitted (basically the very end).
    /// Default impl does nothing.
    fn end_main(&mut self) {}

    /// Emits a comment describing that this field can be loaded from the given
    /// env var. Default impl is likely sufficient.
    fn env_comment(&mut self, env_key: &'static str) {
        self.comment(format_args!(" Can also be specified via environment variable `{env_key}`."));
    }

    /// Emits a comment either stating that this field is required, or
    /// specifying the default value. Default impl is likely sufficient.
    fn default_or_required_comment(&mut self, default_value: Option<&'static Expr>) {
        match default_value {
            None => self.comment(format_args!(" Required! This value must be specified.")),
            Some(v) => self.comment(format_args!(" Default value: {}", Self::ExprPrinter::from(v))),
        }
    }

    /// Makes sure that there is a gap of at least `size` many empty lines at
    /// the end of the buffer. Does nothing when the buffer is empty.
    fn make_gap(&mut self, size: u8) {
        if !self.buffer().is_empty() {
            let num_trailing_newlines = self.buffer().chars()
                .rev()
                .take_while(|c| *c == '\n')
                .count();

            let newlines_needed = (size as usize + 1).saturating_sub(num_trailing_newlines);
            let buffer = self.buffer();
            for _ in 0..newlines_needed {
                buffer.push('\n');
            }
        }
    }

    /// Makes sure the buffer ends with a single trailing newline.
    fn assert_single_trailing_newline(&mut self) {
        let buffer = self.buffer();
        if buffer.ends_with('\n') {
            while buffer.ends_with("\n\n") {
                buffer.pop();
            }
        } else {
            buffer.push('\n');
        }
    }
}

/// General (non format-dependent) template-formatting options.
#[non_exhaustive]
pub struct FormatOptions {
    /// Whether to include doc comments (with your own text and information
    /// about whether a value is required and/or has a default). Default:
    /// `true`.
    pub comments: bool,

    /// If `comments` and this field are `true`, leaf fields with `env = "FOO"`
    /// attribute will have a line like this added:
    ///
    /// ```text
    /// ## Can also be specified via environment variable `FOO`.
    /// ```
    ///
    /// Default: `true`.
    pub env_keys: bool,

    /// Number of lines between leaf fields. Gap between leaf and nested fields
    /// is the bigger of this and `nested_field_gap`.
    ///
    /// Default: `if self.comments { 1 } else { 0 }`.
    pub leaf_field_gap: Option<u8>,

    /// Number of lines between nested fields. Gap between leaf and nested
    /// fields is the bigger of this and `leaf_field_gap`.
    ///
    /// Default: 1.
    pub nested_field_gap: u8,

    // Potential future options:
    // - Comment out default values (`#foo = 3` vs `foo = 3`)
    // - Which docs to include from nested objects
}

impl FormatOptions {
    fn leaf_field_gap(&self) -> u8 {
        self.leaf_field_gap.unwrap_or(self.comments as u8)
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            comments: true,
            env_keys: true,
            leaf_field_gap: None,
            nested_field_gap: 1,
        }
    }
}

/// Formats a configuration template with the given formatter.
///
/// If you don't need to use a custom formatter, rather look at the `format`
/// functions in the format-specific modules (e.g. `toml::format`,
/// `yaml::format`).
pub(crate) fn format<C: Config>(out: &mut impl Formatter, options: FormatOptions) {
    let meta = &C::META;

    // Print root docs.
    if options.comments {
        meta.doc.iter().for_each(|doc| out.comment(doc));
    }

    // Recursively format all nested objects and fields
    out.start_main();
    format_impl(out, meta, &options);
    out.end_main();
    out.assert_single_trailing_newline();
}


fn format_impl(out: &mut impl Formatter, meta: &Meta, options: &FormatOptions) {
    // Output all leaf fields first
    let leaf_fields = meta.fields.iter().filter_map(|f| match &f.kind {
        FieldKind::Leaf { kind, env } => Some((f, kind, env)),
        _ => None,
    });
    let mut emitted_anything = false;
    for (i, (field, kind, env)) in leaf_fields.enumerate() {
        emitted_anything = true;

        if i > 0 {
            out.make_gap(options.leaf_field_gap());
        }

        let mut emitted_something = false;
        macro_rules! empty_sep_doc_line {
            () => {
                if emitted_something {
                    out.comment("");
                }
            };
        }

        if options.comments {
            field.doc.iter().for_each(|doc| out.comment(doc));
            emitted_something = !field.doc.is_empty();

            if let Some(env) = env {
                empty_sep_doc_line!();
                out.env_comment(env);
            }
        }

        match kind {
            LeafKind::Optional => out.disabled_field(field.name, None),
            LeafKind::Required { default } => {
                // Emit comment about default value or the value being required.
                if options.comments {
                    empty_sep_doc_line!();
                    out.default_or_required_comment(default.as_ref())
                }

                // Emit the actual line with the name and optional value
                out.disabled_field(field.name, default.as_ref());
            }
        }
    }

    // Then all nested fields recursively
    let nested_fields = meta.fields.iter().filter_map(|f| match &f.kind {
        FieldKind::Nested { meta } => Some((f, meta)),
        _ => None,
    });
    for (field, meta) in nested_fields {
        if emitted_anything {
            out.make_gap(options.nested_field_gap);
        }
        emitted_anything = true;

        let comments = if options.comments { field.doc } else { &[] };
        out.start_nested(&field.name, comments);
        format_impl(out, meta, options);
        out.end_nested();
    }
}
