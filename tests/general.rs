use std::{collections::HashMap, net::IpAddr, path::PathBuf};
use pretty_assertions::assert_eq;

use confique::{meta, Config, Partial};


#[test]
fn simple() {
    /// Root doc comment banana.
    #[derive(Config)]
    #[allow(dead_code)]
    struct Animals {
        /// Doc comment for cat.
        #[config(default = 8080)]
        cat: u32,

        /// Doc comment for dog.
        dog: String,
    }

    assert_eq!(Animals::META, meta::Meta {
        name: "Animals",
        doc: &[" Root doc comment banana."],
        fields: &[
            meta::Field {
                name: "cat",
                doc: &[" Doc comment for cat."],
                kind: meta::FieldKind::Leaf {
                    env: None,
                    kind: meta::LeafKind::Required {
                        default: Some(meta::Expr::Integer(meta::Integer::U32(8080))),
                    },
                },
            },
            meta::Field {
                name: "dog",
                doc: &[" Doc comment for dog."],
                kind: meta::FieldKind::Leaf {
                    env: None,
                    kind: meta::LeafKind::Required {
                        default: None,
                    },
                },
            },
        ],
    });

    let def = <Animals as Config>::Partial::default_values();
    assert_eq!(def.cat, Some(8080));
    assert_eq!(def.dog, None);
}

mod full {
    #![allow(dead_code)]

    use super::*;

    /// A sample configuration for our app.
    #[derive(Config)]
    pub(crate) struct Conf {
        /// Leaf field on top level struct.
        app_name: String,

        #[config(nested)]
        normal: NormalTest,

        #[config(nested)]
        deserialize_with: DeserializeWithTest,

        /// Doc comment on nested.
        #[config(nested)]
        env: EnvTest,
    }

    #[derive(Config)]
    pub(crate) struct NormalTest {
        required: String,

        #[config(default = "127.0.0.1")]
        with_default: IpAddr,

        optional: Option<PathBuf>,
    }

    /// Testing the `deserialize_with` attribute!
    /// Multiline, wow!
    #[derive(Config)]
    pub(crate) struct DeserializeWithTest {
        #[config(deserialize_with = deserialize_dummy)]
        required: Dummy,

        #[config(deserialize_with = deserialize_dummy, default = "peter")]
        with_default: Dummy,

        #[config(deserialize_with = deserialize_dummy)]
        optional: Option<Dummy>,

        #[config(env = "ENV_TEST_FULL_0", deserialize_with = deserialize_dummy)]
        with_env: Dummy,
    }

    /// Doc comment on nested struct!
    #[derive(Config)]
    pub(crate) struct EnvTest {
        #[config(env = "ENV_TEST_FULL_1")]
        required: String,

        #[config(env = "ENV_TEST_FULL_2", default = 8080)]
        with_default: u16,

        #[config(env = "ENV_TEST_FULL_3")]
        optional: Option<PathBuf>,
    }
}

#[test]
fn full() {
    use full::*;

    assert_eq!(Conf::META, meta::Meta {
        name: "Conf",
        doc: &[" A sample configuration for our app."],
        fields: &[
            meta::Field {
                name: "app_name",
                doc: &[" Leaf field on top level struct."],
                kind: meta::FieldKind::Leaf {
                    env: None,
                    kind: meta::LeafKind::Required { default: None },
                },
            },
            meta::Field {
                name: "normal",
                doc: &[],
                kind: meta::FieldKind::Nested {
                    meta: &meta::Meta {
                        name: "NormalTest",
                        doc: &[],
                        fields: &[
                            meta::Field {
                                name: "required",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: None,
                                    kind: meta::LeafKind::Required { default: None },
                                },
                            },
                            meta::Field {
                                name: "with_default",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: None,
                                    kind: meta::LeafKind::Required {
                                        default: Some(meta::Expr::Str("127.0.0.1")),
                                    },
                                },
                            },
                            meta::Field {
                                name: "optional",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: None,
                                    kind: meta::LeafKind::Optional,
                                },
                            },
                        ],
                    },
                },
            },
            meta::Field {
                name: "deserialize_with",
                doc: &[],
                kind: meta::FieldKind::Nested {
                    meta: &meta::Meta {
                        name: "DeserializeWithTest",
                        doc: &[" Testing the `deserialize_with` attribute!", " Multiline, wow!"],
                        fields: &[
                            meta::Field {
                                name: "required",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: None,
                                    kind: meta::LeafKind::Required { default: None },
                                },
                            },
                            meta::Field {
                                name: "with_default",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: None,
                                    kind: meta::LeafKind::Required {
                                        default: Some(meta::Expr::Str("peter")),
                                    },
                                },
                            },
                            meta::Field {
                                name: "optional",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: None,
                                    kind: meta::LeafKind::Optional,
                                },
                            },
                            meta::Field {
                                name: "with_env",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: Some("ENV_TEST_FULL_0"),
                                    kind: meta::LeafKind::Required { default: None },
                                },
                            },
                        ]
                    },
                },
            },
            meta::Field {
                name: "env",
                doc: &[" Doc comment on nested."],
                kind: meta::FieldKind::Nested {
                    meta: &meta::Meta {
                        name: "EnvTest",
                        doc: &[" Doc comment on nested struct!"],
                        fields: &[
                            meta::Field {
                                name: "required",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: Some("ENV_TEST_FULL_1"),
                                    kind: meta::LeafKind::Required { default: None },
                                },
                            },
                            meta::Field {
                                name: "with_default",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: Some("ENV_TEST_FULL_2"),
                                    kind: meta::LeafKind::Required {
                                        default: Some(meta::Expr::Integer(meta::Integer::U16(8080))),
                                    },
                                },
                            },
                            meta::Field {
                                name: "optional",
                                doc: &[],
                                kind: meta::FieldKind::Leaf {
                                    env: Some("ENV_TEST_FULL_3"),
                                    kind: meta::LeafKind::Optional,
                                },
                            },
                        ],
                    },
                },
            },
        ],
    });

    let def = <Conf as Config>::Partial::default_values();
    assert_eq!(def.app_name, None);
    assert_eq!(def.normal.required, None);
    assert_eq!(def.normal.with_default, Some(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)));
    assert_eq!(def.normal.optional, None);
    assert_eq!(def.deserialize_with.required, None);
    assert_eq!(def.deserialize_with.with_default, Some(Dummy("dummy peter".into())));
    assert_eq!(def.deserialize_with.optional, None);
    assert_eq!(def.deserialize_with.with_env, None);
    assert_eq!(def.env.required, None);
    assert_eq!(def.env.with_default, Some(8080));
    assert_eq!(def.env.optional, None);
}

#[derive(Debug, PartialEq)]
struct Dummy(String);

pub(crate) fn deserialize_dummy<'de, D>(deserializer: D) -> Result<Dummy, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let s = String::deserialize(deserializer)?;
    Ok(Dummy(format!("dummy {s}")))
}

// This only makes sure this compiles and doesn't result in any "cannot infer
// type" problems.
#[test]
fn empty_array_and_map() {
    #[derive(Config)]
    #[allow(dead_code)]
    struct Animals {
        #[config(default = [])]
        cat: Vec<String>,

        #[config(default = {})]
        dog: HashMap<u32, f32>,
    }
}
