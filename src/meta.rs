//! Types for [`Config::META`][super::Config::META]. Represent information about
//! a configuration type.

// TODO: having all these fields public make me uncomfortable. For now it's
// fine, but before reaching 1.0 I need to figure out how to allow future
// additions without breaking stuff.

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    /// The type (struct) name.
    pub name: &'static str,

    /// Doc comments.
    pub doc: &'static [&'static str],

    pub fields: &'static [Field],
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub name: &'static str,
    pub doc: &'static [&'static str],
    pub optional: bool,
    pub kind: FieldKind,
}

#[derive(Clone, Copy, Debug)]
pub enum FieldKind {
    Leaf {
        default: Option<Expr>,
    },
    Nested {
        meta: &'static Meta,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Expr {
    Str(&'static str),
    Float(Float),
    Integer(Integer),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Float {
    F32(f32),
    F64(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integer {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
}
