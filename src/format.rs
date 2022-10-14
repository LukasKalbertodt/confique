use std::fmt;


/// Adds zero, one or two line breaks to make sure that there are at least two
/// line breaks at the end of the string. Except if the buffer is completely
/// empty, in which case it is not modified.
pub(crate) fn add_empty_line(out: &mut String) {
    match () {
        () if out.is_empty() => {},
        () if out.ends_with("\n\n") => {},
        () if out.ends_with('\n') => out.push('\n'),
        _ => out.push_str("\n\n"),
    }
}

pub(crate) fn assert_single_trailing_newline(out: &mut String) {
    while out.ends_with("\n\n") {
        out.pop();
    }
}

pub(crate) struct DefaultValueComment<T>(pub(crate) Option<T>);

impl<T: fmt::Display> fmt::Display for DefaultValueComment<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            None => "Required! This value must be specified.".fmt(f),
            Some(v) => write!(f, "Default value: {v}"),
        }
    }
}
