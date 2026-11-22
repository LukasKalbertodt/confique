//! The generated layer type as public API: nameable and usable from outside
//! according to the config's visibility, and paths in attributes resolve
//! relative to the module the config is defined in.

#![allow(dead_code)]

use confique::{Config, Layer};


/// Checks that `$layer` names the layer type of `$ty` (which needs a validated
/// `x: u32`) and accesses it: struct literal, field access, `validate_x`,
/// `deserialize_x` and `from_layer`.
macro_rules! check_layer {
    ($ty:ty, $layer:ty) => {{
        type Lay = $layer;
        let _: fn(<$ty as confique::Config>::Layer) -> Lay = |l| l;
        let layer = Lay { x: Some(3) };
        assert_eq!(layer.x, Some(3));
        assert!(Lay::validate_x(&3).is_ok());
        assert!(Lay::validate_x(&0).is_err());
        assert_eq!(Lay::deserialize_x(crate::u32_deserializer(3)), Ok(3));
        assert!(Lay::deserialize_x(crate::u32_deserializer(0)).is_err());
        assert!(<$ty as confique::Config>::from_layer(layer).is_ok());
    }};
}

fn u32_deserializer(v: u32) -> serde::de::value::U32Deserializer<serde::de::value::Error> {
    serde::de::IntoDeserializer::into_deserializer(v)
}


// ===== Visibility =====

mod a {
    pub mod b {
        use confique::Config;

        #[derive(Config)]
        pub struct Pub {
            #[config(validate(*x > 0, "zero"))]
            x: u32,
        }

        #[derive(Config)]
        pub(crate) struct Crate {
            #[config(validate(*x > 0, "zero"))]
            x: u32,
        }

        #[derive(Config)]
        pub(super) struct Super {
            #[config(validate(*x > 0, "zero"))]
            x: u32,
        }

        #[derive(Config)]
        pub(in crate::a) struct InA {
            #[config(validate(*x > 0, "zero"))]
            x: u32,
        }

        #[derive(Config)]
        struct Private {
            #[config(validate(*x > 0, "zero"))]
            x: u32,
        }

        pub fn check_private() {
            check_layer!(Private, PrivateLayer);
        }
    }

    pub fn check_restricted() {
        check_layer!(b::Super, b::SuperLayer);
        check_layer!(b::InA, b::InALayer);
    }
}

#[test]
fn visibility() {
    check_layer!(a::b::Pub, a::b::PubLayer);
    check_layer!(a::b::Crate, a::b::CrateLayer);
    a::check_restricted();
    a::b::check_private();
}


// ===== Paths in attributes =====

fn positive(x: &u32) -> Result<(), &'static str> {
    if *x > 0 { Ok(()) } else { Err("zero") }
}

fn deser_double<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    <u32 as serde::Deserialize>::deserialize(d).map(|x| x * 2)
}

fn parse_double(s: &str) -> Result<u32, std::num::ParseIntError> {
    s.parse::<u32>().map(|x| x * 2)
}

mod paths {
    use confique::Config;

    fn local_positive(x: &u32) -> Result<(), &'static str> {
        super::positive(x)
    }

    #[derive(Config)]
    pub(crate) struct Conf {
        #[config(validate = self::local_positive)]
        pub(crate) a: u32,

        #[config(validate = super::positive, deserialize_with = super::deser_double)]
        pub(crate) b: u32,

        #[config(env = "LAYER_TYPE_PATHS_C", parse_env = super::parse_double)]
        pub(crate) c: u32,

        #[config(deserialize_with = super::deser_double)]
        pub(crate) d: u32,
    }
}

#[test]
fn attribute_paths() {
    type Lay = <paths::Conf as Config>::Layer;
    assert!(Lay::validate_a(&0).is_err());
    assert!(Lay::validate_b(&0).is_err());
    assert_eq!(Lay::deserialize_b(u32_deserializer(2)), Ok(4));
    assert!(Lay::deserialize_b(u32_deserializer(0)).is_err());
    assert_eq!(Lay::deserialize_d(u32_deserializer(3)), Ok(6));

    std::env::set_var("LAYER_TYPE_PATHS_C", "5");
    assert_eq!(Lay::from_env().unwrap().c, Some(10));
}


// ===== Items local to a function body =====

#[test]
fn in_fn_body() {
    fn local_positive(x: &u32) -> Result<(), &'static str> {
        if *x > 0 { Ok(()) } else { Err("zero") }
    }

    #[derive(Config)]
    struct Outer {
        #[config(nested)]
        inner: Inner,
    }

    #[derive(Config)]
    struct Inner {
        #[config(default = 1, validate = local_positive)]
        x: u32,
    }

    let conf = Outer::builder().load().unwrap();
    assert_eq!(conf.inner.x, 1);
}


// ===== Custom layer name =====

#[derive(Config)]
#[config(layer_name = Partial)]
struct Named {
    #[config(validate(*x > 0, "zero"), deserialize_with = deser_double)]
    x: u32,

    #[config(nested)]
    nested: RawNamed,
}

#[derive(Config)]
#[config(layer_name = r#Loop)]
struct RawNamed {
    #[config(default = 1)]
    y: u32,
}

#[test]
fn layer_name() {
    let layer = Partial { x: Some(2), nested: Loop::empty() };
    assert!(Partial::validate_x(&0).is_err());
    assert_eq!(Loop::deserialize_y(u32_deserializer(5)), Ok(5));
    assert!(Loop::validate_y(&0).is_ok()); // no validators -> no-op
    let conf = Named::from_layer(layer.with_fallback(Partial::default_values())).unwrap();
    assert_eq!((conf.x, conf.nested.y), (2, 1));
}


// ===== Lints =====

/// Generated items must not trigger lints in the user's crate, e.g. a missing
/// doc on the (public) layer type or its methods.
#[deny(missing_docs)]
pub mod lints {
    /// Config.
    #[derive(confique::Config)]
    pub struct Conf {
        /// A validated field.
        #[config(validate(*x > 0, "zero"))]
        pub x: u32,

        /// A nested field.
        #[config(nested)]
        pub nested: Nested,
    }

    /// Nested config.
    #[derive(confique::Config)]
    pub struct Nested {
        /// A field with custom deserialization.
        #[config(deserialize_with = crate::deser_double)]
        pub y: u32,
    }
}
