use serde::Deserialize;
use confique::{Config};

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
