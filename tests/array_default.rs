use pretty_assertions::assert_eq;
use confique::{Config, meta};


#[test]
fn vec_u32() {
    #[derive(Config)]
    struct Foo {
        /// A nice doc comment.
        #[config(default = [1, 2, 3])]
        bar: Vec<u32>,
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
                        default: Some(meta::Expr::Array(&[
                            meta::Expr::Integer(meta::Integer::U32(1)),
                            meta::Expr::Integer(meta::Integer::U32(2)),
                            meta::Expr::Integer(meta::Integer::U32(3)),
                        ])),
                    },
                },
            },
        ],
    });

    let def = Foo::builder().load().unwrap();
    assert_eq!(def.bar, vec![1, 2, 3]);
}

#[test]
#[allow(unused_parens)]
fn inferred_type() {
    #[derive(Config)]
    struct Foo {
        #[config(default = [1, 2])]
        array: [i8; 2],

        #[config(default = [1, 2])]
        linked_list: std::collections::LinkedList<usize>,

        #[config(default = [1.0, 2.0])]
        parens: (Vec<f32>),

        // A type from which we cannot correctly infer the item type.
        #[config(default = [13, 27])]
        fallback: std::time::Duration,
    }

    #[track_caller]
    fn assert_helper(
        actual: &meta::FieldKind,
        expected_items: &[meta::Expr],
    ) {
        match actual {
            meta::FieldKind::Leaf {
                env: None,
                kind: meta::LeafKind::Required {
                    default: Some(meta::Expr::Array(items)),
                },
            } => {
                assert_eq!(*items, expected_items);
            }
            _ => panic!("expected required leaf field, found: {actual:?}"),
        }
    }

    assert_helper(&Foo::META.fields[0].kind,
        &[1, 2].map(|i| meta::Expr::Integer(meta::Integer::I8(i)))
    );
    assert_helper(&Foo::META.fields[1].kind,
        &[1, 2].map(|i| meta::Expr::Integer(meta::Integer::Usize(i)))
    );
    assert_helper(&Foo::META.fields[2].kind,
        &[1.0, 2.0].map(|f| meta::Expr::Float(meta::Float::F32(f)))
    );
    assert_helper(&Foo::META.fields[3].kind,
        &[13, 27].map(|i| meta::Expr::Integer(meta::Integer::I32(i)))
    );

    let def = Foo::builder().load().unwrap();
    assert_eq!(def.array, [1, 2]);
    assert_eq!(def.linked_list, std::collections::LinkedList::from_iter([1, 2]));
    assert_eq!(def.parens, vec![1.0, 2.0]);
    assert_eq!(def.fallback, std::time::Duration::new(13, 27));
}
