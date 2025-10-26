use serde::Deserialize;
use confique::{Config, Layer};
use pretty_assertions::assert_eq;

#[derive(Debug, Deserialize)]
enum Foo { A, B, C }


#[test]
fn enum_env() {
    #[derive(Config)]
    struct Conf {
        #[config(env = "FOO")]
        foo: Foo,
    }

    std::env::set_var("FOO", "B");
    let conf = Conf::builder().env().load();
    assert!(matches!(conf, Ok(Conf { foo: Foo::B })));
}

fn my_parser(s: &str) -> Result<u32, impl std::error::Error> {
    s.trim().parse()
}

fn my_parser2(s: &str) -> Result<u32, impl std::error::Error> {
    if s.is_empty() {
        Ok(0)
    } else {
        s.trim().parse()
    }
}

#[test]
fn empty_error_is_unset() {
    #[derive(Config)]
    #[config(layer_attr(derive(PartialEq, Debug)))]
    #[allow(dead_code)]
    struct Conf {
        #[config(env = "EMPTY_ERROR_IS_UNSET_FOO")]
        foo: u32,

        #[config(env = "EMPTY_ERROR_IS_UNSET_BAR", parse_env = my_parser)]
        bar: u32,

        #[config(env = "EMPTY_ERROR_IS_UNSET_BAZ")]
        baz: String,

        #[config(env = "EMPTY_ERROR_IS_UNSET_VALIDATE", validate(!validate.is_empty(), "bad"))]
        validate: String,

        #[config(
            env = "EMPTY_ERROR_IS_UNSET_VALIDATE_PARSE",
            parse_env = my_parser2,
            validate(*validate_parse != 0, "bad"),
        )]
        validate_parse: u32,
    }

    type Layer = <Conf as Config>::Layer;

    std::env::set_var("EMPTY_ERROR_IS_UNSET_FOO", "");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        foo: None,
        bar: None,
        baz: None,
        validate: None,
        validate_parse: None,
    });

    std::env::set_var("EMPTY_ERROR_IS_UNSET_BAR", "");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        foo: None,
        bar: None,
        baz: None,
        validate: None,
        validate_parse: None,
    });

    std::env::set_var("EMPTY_ERROR_IS_UNSET_BAZ", "");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        foo: None,
        bar: None,
        baz: Some("".into()),
        validate: None,
        validate_parse: None,
    });

    std::env::set_var("EMPTY_ERROR_IS_UNSET_VALIDATE", "");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        foo: None,
        bar: None,
        baz: Some("".into()),
        validate: None,
        validate_parse: None,
    });

    std::env::set_var("EMPTY_ERROR_IS_UNSET_VALIDATE_PARSE", "");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        foo: None,
        bar: None,
        baz: Some("".into()),
        validate: None,
        validate_parse: None,
    });
}
