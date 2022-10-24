use std::collections::HashMap;
use pretty_assertions::assert_eq;

use confique::{meta, Config};


#[test]
fn string_to_u32() {
    #[derive(Config)]
    struct Foo {
        /// A nice doc comment.
        #[config(default = { "peter": 3, "anna": 27 })]
        bar: HashMap<String, u32>,
    }

    assert_eq!(Foo::META, meta::Meta {
        name: "Foo",
        doc: &[],
        fields: &[
            meta::Field {
                name: "bar",
                doc: &[" A nice doc comment."],
                kind: meta::FieldKind::Leaf {
                    env: None,
                    kind: meta::LeafKind::Required {
                        default: Some(meta::Expr::Map(&[
                            meta::MapEntry {
                                key: meta::MapKey::Str("peter"),
                                value: meta::Expr::Integer(meta::Integer::U32(3)),
                            },
                            meta::MapEntry {
                                key: meta::MapKey::Str("anna"),
                                value: meta::Expr::Integer(meta::Integer::U32(27)),
                            },
                        ])),
                    },
                },
            },
        ],
    });

    let def = Foo::builder().load().unwrap();
    assert_eq!(def.bar, HashMap::from([("peter".into(), 3), ("anna".into(), 27)]));
}
