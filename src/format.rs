//! Utilities for formatting configuration templates and serializing configs.
//!
//! This module provides the infrastructure for both:
//! - Generating config templates (all fields disabled, showing defaults)
//! - Serializing config instances (fields enabled/disabled based on values)

use std::fmt;

use crate::meta::{Expr, FieldKind, LeafKind, Meta};

/// Trait abstracting over format differences when formatting configuration output.
///
/// This trait handles both template generation and config serialization.
pub(crate) trait ConfigFormatter {
    /// Internal buffer, mainly used for `make_gap` and similar methods.
    fn buffer(&mut self) -> &mut String;

    /// Returns internal buffer by value.
    fn finish(self) -> String;

    /// Write a comment, e.g. `format!("#{comment}")`. Don't add a space after
    /// your comment token.
    fn comment(&mut self, comment: impl fmt::Display);

    /// Write an enabled field with a value.
    fn field(&mut self, name: &str, value: &str);

    /// Write a disabled (commented-out) field with an optional value.
    fn disabled_field(&mut self, name: &str, value: Option<&str>);

    /// Start a nested configuration section with the given name.
    fn start_nested(&mut self, name: &str, doc: &[&str]);

    /// End a nested configuration section.
    fn end_nested(&mut self);

    /// Push a path segment without emitting a section header.
    /// Used for table-of-tables rendering where the parent key must be part
    /// of the path but should not produce its own header.
    fn push_path(&mut self, _name: &str) {}

    /// Pop a path segment previously pushed by [`push_path`].
    fn pop_path(&mut self) {}

    /// Called after the global docs are written and before any fields are
    /// emitted. Default impl does nothing.
    fn start_main(&mut self) {}

    /// Called after all fields have been emitted (basically the very end).
    /// Default impl does nothing.
    fn end_main(&mut self) {}

    /// Emits a comment describing that this field can be loaded from the given
    /// env var. Default impl is likely sufficient.
    fn env_comment(&mut self, env_key: &'static str) {
        self.comment(format_args!(
            " Can also be specified via environment variable `{env_key}`."
        ));
    }

    /// Emits a comment either stating that this field is required, or
    /// specifying the default value. Used in template mode.
    fn default_or_required_comment(&mut self, formatted_default: Option<&str>) {
        match formatted_default {
            None => self.comment(format_args!(" Required! This value must be specified.")),
            Some(v) => self.comment(format_args!(" Default value: {v}")),
        }
    }

    /// Emits a comment specifying the default value. Used in serialize mode.
    /// Only emits if there is a default value.
    fn default_hint(&mut self, formatted_default: Option<&str>) {
        if let Some(v) = formatted_default {
            self.comment(format_args!(" Default value: {v}"));
        }
    }

    /// Makes sure that there is a gap of at least `size` many empty lines at
    /// the end of the buffer. Does nothing when the buffer is empty.
    fn make_gap(&mut self, size: u8) {
        if !self.buffer().is_empty() {
            let num_trailing_newlines = self
                .buffer()
                .chars()
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

/// Trait for accessing values from a serialized intermediate representation.
///
/// This abstracts over `toml::Value`, `serde_yaml::Value`, `serde_json::Value`, etc.
#[cfg(feature = "serialize")]
pub(crate) trait ValueAccess {
    /// Get a nested field by name. Returns `None` if the field doesn't exist
    /// or if `self` is not a table/map type.
    fn get_field(&self, name: &str) -> Option<&Self>;

    /// Format this value as a string suitable for the target format.
    fn format_value(&self) -> String;

    /// Returns true if this value represents null/None.
    /// Default returns false; override for formats that serialize None as null.
    fn is_null(&self) -> bool {
        false
    }

    /// Returns the entries of a table value as `(key, value)` pairs.
    /// Returns `None` if this value is not a table.
    fn table_entries(&self) -> Option<Vec<(&str, &Self)>> {
        None
    }

    /// If this value is a non-empty table whose values are all themselves
    /// tables, returns the entries as `(key, value)` pairs. Otherwise returns
    /// `None`. Used to expand map-of-struct leaf fields as TOML sections
    /// instead of inline tables.
    fn nested_table_entries(&self) -> Option<Vec<(&str, &Self)>> {
        None
    }
}

/// General (non format-dependent) formatting options.
#[non_exhaustive]
pub struct FormatOptions {
    /// Whether to include doc comments (with your own text and information
    /// about whether a value is required and/or has a default). Default: `true`.
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
}

impl FormatOptions {
    pub(crate) fn leaf_field_gap(&self) -> u8 {
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
/// All fields are disabled (commented out) and show their
/// default values. Fields without defaults show "Required!" message.
pub(crate) fn template<F>(
    meta: &Meta,
    out: &mut F,
    options: &FormatOptions,
    expr_printer: fn(&Expr) -> String,
) where
    F: ConfigFormatter,
{
    // Print root docs.
    if options.comments {
        meta.doc.iter().for_each(|doc| out.comment(doc));
    }

    // Recursively format all nested objects and fields
    out.start_main();
    template_impl(out, meta, options, expr_printer);
    out.end_main();
    out.assert_single_trailing_newline();
}

fn template_impl<F>(
    out: &mut F,
    meta: &Meta,
    options: &FormatOptions,
    expr_printer: fn(&Expr) -> String,
) where
    F: ConfigFormatter,
{
    // Output all leaf fields first
    let leaf_fields = meta.fields.iter()
        .filter(|f| !f.skip)
        .filter_map(|f| match &f.kind {
            FieldKind::Leaf { kind, env } => Some((f, kind, env)),
            _ => None,
        });
    let mut emitted_anything = false;
    for (field, kind, env) in leaf_fields {
        if emitted_anything {
            out.make_gap(options.leaf_field_gap());
        }
        emitted_anything = true;

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

            if options.env_keys {
                if let Some(env) = env {
                    empty_sep_doc_line!();
                    out.env_comment(env);
                    emitted_something = true;
                }
            }
        }

        match kind {
            LeafKind::Optional => {
                // Optional field with no default - just show field name
                out.disabled_field(field.name, None);
            }
            LeafKind::Required { default } => {
                let formatted_default = default.as_ref().map(expr_printer);

                // Emit comment about default value or the value being required.
                if options.comments {
                    empty_sep_doc_line!();
                    out.default_or_required_comment(formatted_default.as_deref());
                }

                // Emit the disabled field with optional value
                out.disabled_field(field.name, formatted_default.as_deref());
            }
        }
    }

    // Then all nested fields recursively
    let nested_fields = meta.fields.iter()
        .filter(|f| !f.skip)
        .filter_map(|f| match &f.kind {
            FieldKind::Nested { meta } => Some((f, meta)),
            _ => None,
        });
    for (field, nested_meta) in nested_fields {
        if emitted_anything {
            out.make_gap(options.nested_field_gap);
        }
        emitted_anything = true;

        let comments = if options.comments { field.doc } else { &[] };
        out.start_nested(field.name, comments);
        template_impl(out, nested_meta, options, expr_printer);
        out.end_nested();
    }
}

/// Serializes a configuration with the given formatter.
///
/// Fields are disabled (commented out) if their value equals
/// the default, or if they are optional with no value. Other fields are enabled.
#[cfg(feature = "serialize")]
pub(crate) fn serialize<F, V>(
    meta: &Meta,
    values: &V,
    out: &mut F,
    options: &FormatOptions,
    expr_printer: fn(&Expr) -> String,
) where
    F: ConfigFormatter,
    V: ValueAccess,
{
    // Print root docs.
    if options.comments {
        meta.doc.iter().for_each(|doc| out.comment(doc));
    }

    // Recursively format all nested objects and fields
    out.start_main();
    serialize_impl(out, meta, values, options, expr_printer);
    out.end_main();
    out.assert_single_trailing_newline();
}

#[cfg(feature = "serialize")]
fn serialize_impl<F, V>(
    out: &mut F,
    meta: &Meta,
    values: &V,
    options: &FormatOptions,
    expr_printer: fn(&Expr) -> String,
) where
    F: ConfigFormatter,
    V: ValueAccess,
{
    // Output all leaf fields first
    let leaf_fields = meta.fields.iter()
        .filter(|f| !f.skip)
        .filter_map(|f| match &f.kind {
            FieldKind::Leaf { kind, env } => Some((f, kind, env)),
            _ => None,
        });
    let mut emitted_anything = false;
    for (field, kind, env) in leaf_fields {
        // Get the value for this field from the serialized config
        let field_value = values.get_field(field.name);

        // If the leaf value is a table-of-tables, expand it as nested sections
        // instead of rendering inline. This handles map fields like
        // HashMap<String, SomeStruct> which serialize to nested TOML tables.
        if let Some(outer_entries) = field_value.and_then(|fv| fv.nested_table_entries()) {
            if emitted_anything {
                out.make_gap(options.nested_field_gap);
            }
            emitted_anything = true;

            if options.comments {
                field.doc.iter().for_each(|doc| out.comment(doc));
                if options.env_keys {
                    if let Some(env) = env {
                        out.env_comment(env);
                    }
                }
            }

            out.push_path(field.name);
            let mut first = true;
            for (key, inner_table) in &outer_entries {
                if !first {
                    out.make_gap(options.leaf_field_gap());
                }
                first = false;
                out.start_nested(key, &[]);
                if let Some(inner_entries) = inner_table.table_entries() {
                    for (inner_key, inner_value) in &inner_entries {
                        out.field(inner_key, &inner_value.format_value());
                    }
                }
                out.end_nested();
            }
            out.pop_path();
            continue;
        }

        // Get the default value
        let default_value = match kind {
            LeafKind::Required { default } => default.as_ref(),
            LeafKind::Optional => None,
        };

        // Format values
        let formatted_value = field_value.map(|v| v.format_value());
        let formatted_default = default_value.map(expr_printer);

        // Determine if this field should be disabled (commented out):
        // - Optional fields with no value (None/null)
        // - Fields whose value equals the default
        let is_disabled = if matches!(kind, LeafKind::Optional)
            && field_value.map(|v| v.is_null()).unwrap_or(true)
        {
            // Optional field with no value
            true
        } else if let (Some(val), Some(def)) = (&formatted_value, &formatted_default) {
            // Value equals default - should be disabled
            val == def
        } else {
            false
        };

        if emitted_anything {
            out.make_gap(options.leaf_field_gap());
        }
        emitted_anything = true;

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

            if options.env_keys {
                if let Some(env) = env {
                    empty_sep_doc_line!();
                    out.env_comment(env);
                    emitted_something = true;
                }
            }
        }

        // Emit comment about default value if configured
        if options.comments && default_value.is_some() {
            empty_sep_doc_line!();
            out.default_hint(formatted_default.as_deref());
        }

        // Emit the field - disabled if value equals default or optional None
        if is_disabled {
            out.disabled_field(field.name, formatted_default.as_deref());
        } else {
            match formatted_value {
                Some(v) => out.field(field.name, &v),
                None => {
                    // Required field with no value - this shouldn't happen for a
                    // valid Config instance, but we handle it gracefully
                    out.field(field.name, "");
                }
            }
        }
    }

    // Then all nested fields recursively
    let nested_fields = meta.fields.iter()
        .filter(|f| !f.skip)
        .filter_map(|f| match &f.kind {
            FieldKind::Nested { meta } => Some((f, meta)),
            _ => None,
        });
    for (field, nested_meta) in nested_fields {
        if emitted_anything {
            out.make_gap(options.nested_field_gap);
        }
        emitted_anything = true;

        let comments = if options.comments { field.doc } else { &[] };
        out.start_nested(field.name, comments);

        // Get nested values, or use an empty fallback
        if let Some(nested_values) = values.get_field(field.name) {
            serialize_impl(out, nested_meta, nested_values, options, expr_printer);
        }

        out.end_nested();
    }
}
