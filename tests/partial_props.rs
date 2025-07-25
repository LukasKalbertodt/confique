use confique::Config;
use pretty_assertions::assert_eq;

#[test]
fn partial_props() {
    #[allow(dead_code)]
    #[derive(Config)]
    #[config(
        partial_attr(derive(derive_more::Debug, derive_more::Display)),
        partial_attr(display("{}", self.bar.unwrap_or_default())),
    )]
    struct Foo {
        #[config(default = 1, partial_attr(debug("test {bar:?}")))]
        bar: u32,
    }

    type PartialFoo = <Foo as Config>::Partial;

    let partial_foo = PartialFoo { bar: Some(1) };
    assert_eq!(
        format!("{partial_foo:?}"),
        "PartialFoo { bar: test Some(1) }",
    );

    assert_eq!(format!("{partial_foo}"), "1");
}

#[test]
fn partial_props_nested() {
    mod foo {
        use confique::Config;

        #[allow(dead_code)]
        #[derive(Config)]
        #[config(partial_attr(derive(derive_more::Debug, derive_more::Deref)))]
        pub struct Foo {
            #[config(default = 1, partial_attr(debug("test {bar:?}")), partial_attr(deref))]
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

    type PartialFoo = <foo::Foo as Config>::Partial;
    type PartialBar = <foo::Bar as Config>::Partial;

    let partial_foo = PartialFoo { bar: Some(1) };
    assert_eq!(*partial_foo, Some(1));

    let partial_bar = PartialBar { foo2: partial_foo };
    assert_eq!(
        format!("{partial_bar:?}"),
        "PartialBar { foo2: test2 PartialFoo { bar: test Some(1) } }",
    );
}
