//! Fields and structs with raw identifiers (e.g. `r#type`). Like serde, the
//! `r#` prefix should not be part of any name at runtime.

use pretty_assertions::assert_eq;

use confique::{Config, Layer};


#[derive(Config)]
#[allow(dead_code)]
struct Conf {
    #[config(env = "RAW_IDENT_TYPE", validate(!r#type.is_empty(), "must not be empty"))]
    r#type: String,

    #[config(nested)]
    r#impl: r#Nested,
}

#[derive(Config)]
#[allow(dead_code)]
struct r#Nested {
    r#loop: u32,
}

#[test]
fn raw_ident() {
    assert_eq!(Conf::META.fields[0].name, "type");
    assert_eq!(Conf::META.fields[1].name, "impl");
    assert_eq!(Nested::META.name, "Nested");
    assert_eq!(Nested::META.fields[0].name, "loop");

    let mut layer = <Conf as Config>::Layer::empty();
    layer.r#type = Some("a".into());
    let err = Conf::from_layer(layer).err().unwrap();
    assert_eq!(err.to_string(), "required configuration value is missing: 'impl.loop'");
}
