use serde::Deserialize;
use confique::{Config, Partial};
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

#[test]
fn empty_error_is_unset() {
    #[derive(Config)]
    #[config(partial_attr(derive(PartialEq, Debug)))]
    #[allow(dead_code)]
    struct Conf {
        #[config(env = "EMPTY_ERROR_IS_UNSET_FOO")]
        foo: u32,

        #[config(env = "EMPTY_ERROR_IS_UNSET_BAR", parse_env = my_parser)]
        bar: u32,

        #[config(env = "EMPTY_ERROR_IS_UNSET_BAZ")]
        baz: String,
    }

    type Partial = <Conf as Config>::Partial;

    std::env::set_var("EMPTY_ERROR_IS_UNSET_FOO", "");
    assert_eq!(Partial::from_env().unwrap(), Partial {
        foo: None,
        bar: None,
        baz: None,
    });

    std::env::set_var("EMPTY_ERROR_IS_UNSET_BAR", "");
    assert_eq!(Partial::from_env().unwrap(), Partial {
        foo: None,
        bar: None,
        baz: None,
    });

    std::env::set_var("EMPTY_ERROR_IS_UNSET_BAZ", "");
    assert_eq!(Partial::from_env().unwrap(), Partial {
        foo: None,
        bar: None,
        baz: Some("".into()),
    });
}
