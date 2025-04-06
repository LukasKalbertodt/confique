use confique::Config;
use pretty_assertions::assert_eq;

#[test]
fn partial_props() {
    #[allow(dead_code)]
    #[derive(Config)]
    #[config(partial_attr(derive(derive_more::Debug)))]
    struct Foo {
        #[config(default = 1, partial_attr(debug("test {bar:?}")))]
        bar: u32,
    }

    use confique_partial_foo::PartialFoo;
    let partial_foo = PartialFoo { bar: Some(1) };
    assert_eq!(
        "PartialFoo { bar: test Some(1) }",
        format!("{partial_foo:?}")
    );
}

#[test]
fn partial_props_nested() {
    mod foo {
        use confique::Config;
        pub use confique_partial_bar::PartialBar;
        pub use confique_partial_foo::PartialFoo;

        #[allow(dead_code)]
        #[derive(Config)]
        #[config(partial_attr(derive(derive_more::Debug)))]
        pub struct Foo {
            #[config(default = 1, partial_attr(debug("test {bar:?}")))]
            bar: u32,
        }

        #[allow(dead_code)]
        #[derive(Config)]
        #[config(partial_attr(derive(derive_more::Debug)))]
        pub struct Bar {
            #[config(nested, partial_attr(debug("test2 {foo2:?}")))]
            foo2: Foo,
        }
    }

    use foo::*;

    let partial_foo = PartialFoo { bar: Some(1) };
    let partial_bar = PartialBar { foo2: partial_foo };
    assert_eq!(
        "PartialBar { foo2: test2 PartialFoo { bar: test Some(1) } }",
        format!("{partial_bar:?}")
    );
}
