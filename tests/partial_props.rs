use confique::Config;
use pretty_assertions::assert_eq;

#[test]
fn layer_props() {
    #[allow(dead_code)]
    #[derive(Config)]
    #[config(
        layer_attr(derive(derive_more::Debug, derive_more::Display)),
        layer_attr(display("{}", self.bar.unwrap_or_default())),
    )]
    struct Foo {
        #[config(default = 1, layer_attr(debug("test {bar:?}")))]
        bar: u32,
    }

    type FooLayer = <Foo as Config>::Layer;

    let foo_layer = FooLayer { bar: Some(1) };
    assert_eq!(
        format!("{foo_layer:?}"),
        "FooLayer { bar: test Some(1) }",
    );

    assert_eq!(format!("{foo_layer}"), "1");
}

#[test]
fn layer_props_nested() {
    mod foo {
        use confique::Config;

        #[allow(dead_code)]
        #[derive(Config)]
        #[config(layer_attr(derive(derive_more::Debug, derive_more::Deref)))]
        pub struct Foo {
            #[config(default = 1, layer_attr(debug("test {bar:?}")), layer_attr(deref))]
            bar: u32,
        }

        #[allow(dead_code)]
        #[derive(Config)]
        #[config(layer_attr(derive(derive_more::Debug)))]
        pub struct Bar {
            #[config(nested, layer_attr(debug("test2 {foo2:?}")))]
            foo2: Foo,
        }
    }

    type FooLayer = <foo::Foo as Config>::Layer;
    type BarLayer = <foo::Bar as Config>::Layer;

    let foo_layer = FooLayer { bar: Some(1) };
    assert_eq!(*foo_layer, Some(1));

    let bar_layer = BarLayer { foo2: foo_layer };
    assert_eq!(
        format!("{bar_layer:?}"),
        "BarLayer { foo2: test2 FooLayer { bar: test Some(1) } }",
    );
}
