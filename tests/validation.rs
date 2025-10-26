use pretty_assertions::assert_eq;

use confique::{Config, Layer};


fn validate_not_1234(foo: &u32) -> Result<(), &'static str> {
    if *foo == 1234 {
        Err("bad password")
    } else {
        Ok(())
    }
}

#[test]
#[should_panic(expected = "default config value for `Conf::foo` cannot be \
    deserialized: Error(\"validation failed: bad password\")")]
fn invalid_default_panics_function() {
    #[derive(Config)]
    #[allow(dead_code)]
    struct Conf {
        #[config(default = 1234, validate = validate_not_1234)]
        foo: u32,
    }

    let _ = <Conf as Config>::Layer::default_values();
}

#[test]
#[should_panic(expected = "default config value for `Conf::foo` cannot be \
    deserialized: Error(\"validation failed: ugly number\")")]
fn invalid_default_panics_assert_like() {
    #[derive(Config)]
    #[allow(dead_code)]
    struct Conf {
        #[config(default = 1234, validate(*foo != 1234, "ugly number"))]
        foo: u32,
    }

    let _ = <Conf as Config>::Layer::default_values();
}

#[test]
fn assert_like() {
    #[derive(Config)]
    #[allow(dead_code)]
    #[config(layer_attr(derive(Debug, PartialEq)))]
    struct Conf {
        #[config(
            env = "AL_REQ",
            validate(req.is_ascii(), "non-ASCII characters ~req are not allowed"),
        )]
        req: String,

        #[config(
            env = "AL_DEF",
            default = "root",
            validate(def.is_ascii(), "non-ASCII characters ~def are not allowed"),
        )]
        def: String,

        #[config(
            env = "AL_OPT",
            validate(opt.is_ascii(), "non-ASCII characters ~opt are not allowed"),
        )]
        opt: Option<String>,


        #[config(
            env = "AL_MULTI",
            validate(multi.is_ascii(), "non-ASCII characters ~multi are not allowed"),
            validate(multi != "banana", "banana ~multi"),
        )]
        multi: String,
    }

    type Layer = <Conf as Config>::Layer;

    // Defaults
    assert_eq!(Layer::default_values(), Layer {
        req: None,
        def: Some("root".into()),
        opt: None,
        multi: None,
    });


    // From env
    std::env::set_var("AL_REQ", "jürgen");
    assert_err_contains(Layer::from_env(), "non-ASCII characters ~req are not allowed");
    std::env::set_var("AL_REQ", "cat");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat".into()),
        def: None,
        opt: None,
        multi: None,
    });

    std::env::set_var("AL_DEF", "I ❤️ fluffy animals");
    assert_err_contains(Layer::from_env(), "non-ASCII characters ~def are not allowed");
    std::env::set_var("AL_DEF", "dog");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat".into()),
        def: Some("dog".into()),
        opt: None,
        multi: None,
    });

    std::env::set_var("AL_OPT", "Μου αρέσουν τα χνουδωτά ζώα");
    assert_err_contains(Layer::from_env(), "non-ASCII characters ~opt are not allowed");
    std::env::set_var("AL_OPT", "fox");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat".into()),
        def: Some("dog".into()),
        opt: Some("fox".into()),
        multi: None,
    });


    std::env::set_var("AL_MULTI", "Μου αρέσουν τα χνουδωτά ζώα");
    assert_err_contains(Layer::from_env(), "non-ASCII characters ~multi are not allowed");
    std::env::set_var("AL_MULTI", "banana");
    assert_err_contains(Layer::from_env(), "banana ~multi");
    std::env::set_var("AL_MULTI", "mouse");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat".into()),
        def: Some("dog".into()),
        opt: Some("fox".into()),
        multi: Some("mouse".into()),
    });


    // From file
    assert_err_contains(
        toml::from_str::<Layer>(r#"req = "jürgen""#),
        "non-ASCII characters ~req are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"def = "I ❤️ fluffy animals""#),
        "non-ASCII characters ~def are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"opt = "Μου αρέσουν τα χνουδωτά ζώα""#),
        "non-ASCII characters ~opt are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"multi = "Μου αρέσουν τα χνουδωτά ζώα""#),
        "non-ASCII characters ~multi are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"multi = "banana""#),
        "banana ~multi",
    );
    assert_eq!(
        toml::from_str::<Layer>("req = \"cat\"\ndef = \"dog\"\nopt = \"fox\"\nmulti = \"mouse\"")
            .unwrap(),
        Layer {
            req: Some("cat".into()),
            def: Some("dog".into()),
            opt: Some("fox".into()),
            multi: Some("mouse".into()),
        },
    );
}

fn assert_is_ascii(s: &String) -> Result<(), &'static str> {
    if !s.is_ascii() {
        Err("non-ASCII characters are not allowed")
    } else {
        Ok(())
    }
}

fn assert_non_banana(s: &String) -> Result<(), &'static str> {
    if s == "banana" {
        Err("banana")
    } else {
        Ok(())
    }
}

#[test]
fn function() {
    #[derive(Config)]
    #[allow(dead_code)]
    #[config(layer_attr(derive(Debug, PartialEq)))]
    struct Conf {
        #[config(env = "FN_REQ", validate = assert_is_ascii)]
        req: String,

        #[config(env = "FN_DEF", default = "root", validate = assert_is_ascii)]
        def: String,

        #[config(env = "FN_OPT", validate = assert_is_ascii)]
        opt: Option<String>,

        #[config(env = "FN_MULTI", validate = assert_is_ascii, validate = assert_non_banana)]
        multi: String,
    }

    type Layer = <Conf as Config>::Layer;

    // Defaults
    assert_eq!(Layer::default_values(), Layer {
        req: None,
        def: Some("root".into()),
        opt: None,
        multi: None,
    });


    // From env
    std::env::set_var("FN_REQ", "jürgen");
    assert_err_contains(Layer::from_env(), "non-ASCII characters are not allowed");
    std::env::set_var("FN_REQ", "cat");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat".into()),
        def: None,
        opt: None,
        multi: None,
    });

    std::env::set_var("FN_DEF", "I ❤️ fluffy animals");
    assert_err_contains(Layer::from_env(), "non-ASCII characters are not allowed");
    std::env::set_var("FN_DEF", "dog");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat".into()),
        def: Some("dog".into()),
        opt: None,
        multi: None,
    });

    std::env::set_var("FN_OPT", "Μου αρέσουν τα χνουδωτά ζώα");
    assert_err_contains(Layer::from_env(), "non-ASCII characters are not allowed");
    std::env::set_var("FN_OPT", "fox");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat".into()),
        def: Some("dog".into()),
        opt: Some("fox".into()),
        multi: None,
    });

    std::env::set_var("FN_MULTI", "Μου αρέσουν τα χνουδωτά ζώα");
    assert_err_contains(Layer::from_env(), "non-ASCII characters are not allowed");
    std::env::set_var("FN_MULTI", "banana");
    assert_err_contains(Layer::from_env(), "banana");
    std::env::set_var("FN_MULTI", "mouse");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat".into()),
        def: Some("dog".into()),
        opt: Some("fox".into()),
        multi: Some("mouse".into()),
    });

    // From file
    assert_err_contains(
        toml::from_str::<Layer>(r#"req = "jürgen""#),
        "non-ASCII characters are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"def = "I ❤️ fluffy animals""#),
        "non-ASCII characters are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"opt = "Μου αρέσουν τα χνουδωτά ζώα""#),
        "non-ASCII characters are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"multi = "Μου αρέσουν τα χνουδωτά ζώα""#),
        "non-ASCII characters are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"multi = "banana""#),
        "banana",
    );
    assert_eq!(
        toml::from_str::<Layer>("req = \"cat\"\ndef = \"dog\"\nopt = \"fox\"\nmulti = \"mouse\"")
            .unwrap(),
        Layer {
            req: Some("cat".into()),
            def: Some("dog".into()),
            opt: Some("fox".into()),
            multi: Some("mouse".into()),
        },
    );
}

fn deserialize_append<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut s = <String as serde::Deserialize>::deserialize(deserializer)?;
    s.push_str("-henlo");
    Ok(s)
}

#[test]
fn assert_like_with_deserializer() {
    #[derive(Config)]
    #[allow(dead_code)]
    #[config(layer_attr(derive(Debug, PartialEq)))]
    struct Conf {
        #[config(
            env = "ALD_REQ",
            deserialize_with = deserialize_append,
            validate(req.is_ascii(), "non-ASCII characters ~req are not allowed"),
        )]
        req: String,

        #[config(
            env = "ALD_DEF",
            default = "root",
            deserialize_with = deserialize_append,
            validate(def.is_ascii(), "non-ASCII characters ~def are not allowed"),
        )]
        def: String,

        #[config(
            env = "ALD_OPT",
            deserialize_with = deserialize_append,
            validate(opt.is_ascii(), "non-ASCII characters ~opt are not allowed"),
        )]
        opt: Option<String>,
    }

    type Layer = <Conf as Config>::Layer;

    // Defaults
    assert_eq!(Layer::default_values(), Layer {
        req: None,
        def: Some("root-henlo".into()),
        opt: None,
    });


    // From env
    std::env::set_var("ALD_REQ", "jürgen");
    assert_err_contains(Layer::from_env(), "non-ASCII characters ~req are not allowed");
    std::env::set_var("ALD_REQ", "cat");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat-henlo".into()),
        def: None,
        opt: None,
    });

    std::env::set_var("ALD_DEF", "I ❤️ fluffy animals");
    assert_err_contains(Layer::from_env(), "non-ASCII characters ~def are not allowed");
    std::env::set_var("ALD_DEF", "dog");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat-henlo".into()),
        def: Some("dog-henlo".into()),
        opt: None,
    });

    std::env::set_var("ALD_OPT", "Μου αρέσουν τα χνουδωτά ζώα");
    assert_err_contains(Layer::from_env(), "non-ASCII characters ~opt are not allowed");
    std::env::set_var("ALD_OPT", "fox");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat-henlo".into()),
        def: Some("dog-henlo".into()),
        opt: Some("fox-henlo".into()),
    });


    // From file
    assert_err_contains(
        toml::from_str::<Layer>(r#"req = "jürgen""#),
        "non-ASCII characters ~req are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"def = "I ❤️ fluffy animals""#),
        "non-ASCII characters ~def are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"opt = "Μου αρέσουν τα χνουδωτά ζώα""#),
        "non-ASCII characters ~opt are not allowed",
    );
    assert_eq!(
        toml::from_str::<Layer>("req = \"cat\"\ndef = \"dog\"\nopt = \"fox\"").unwrap(),
        Layer {
            req: Some("cat-henlo".into()),
            def: Some("dog-henlo".into()),
            opt: Some("fox-henlo".into()),
        },
    );
}

#[test]
fn function_with_deserializer() {
    #[derive(Config)]
    #[allow(dead_code)]
    #[config(layer_attr(derive(Debug, PartialEq)))]
    struct Conf {
        #[config(
            env = "FND_REQ",
            validate = assert_is_ascii,
            deserialize_with = deserialize_append,
        )]
        req: String,

        #[config(
            env = "FND_DEF",
            default = "root",
            validate = assert_is_ascii,
            deserialize_with = deserialize_append,
        )]
        def: String,

        #[config(
            env = "FND_OPT",
            validate = assert_is_ascii,
            deserialize_with = deserialize_append,
        )]
        opt: Option<String>,
    }

    type Layer = <Conf as Config>::Layer;

    // Defaults
    assert_eq!(Layer::default_values(), Layer {
        req: None,
        def: Some("root-henlo".into()),
        opt: None,
    });


    // From env
    std::env::set_var("FND_REQ", "jürgen");
    assert_err_contains(Layer::from_env(), "non-ASCII characters are not allowed");
    std::env::set_var("FND_REQ", "cat");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat-henlo".into()),
        def: None,
        opt: None,
    });

    std::env::set_var("FND_DEF", "I ❤️ fluffy animals");
    assert_err_contains(Layer::from_env(), "non-ASCII characters are not allowed");
    std::env::set_var("FND_DEF", "dog");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat-henlo".into()),
        def: Some("dog-henlo".into()),
        opt: None,
    });

    std::env::set_var("FND_OPT", "Μου αρέσουν τα χνουδωτά ζώα");
    assert_err_contains(Layer::from_env(), "non-ASCII characters are not allowed");
    std::env::set_var("FND_OPT", "fox");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        req: Some("cat-henlo".into()),
        def: Some("dog-henlo".into()),
        opt: Some("fox-henlo".into()),
    });


    // From file
    assert_err_contains(
        toml::from_str::<Layer>(r#"req = "jürgen""#),
        "non-ASCII characters are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"def = "I ❤️ fluffy animals""#),
        "non-ASCII characters are not allowed",
    );
    assert_err_contains(
        toml::from_str::<Layer>(r#"opt = "Μου αρέσουν τα χνουδωτά ζώα""#),
        "non-ASCII characters are not allowed",
    );
    assert_eq!(
        toml::from_str::<Layer>("req = \"cat\"\ndef = \"dog\"\nopt = \"fox\"").unwrap(),
        Layer {
            req: Some("cat-henlo".into()),
            def: Some("dog-henlo".into()),
            opt: Some("fox-henlo".into()),
        },
    );
}

fn validate_vec(v: &Vec<u32>) -> Result<(), &'static str> {
    if v.len() < 3  {
        return Err("list too short");
    }
    Ok(())
}

#[test]
fn parse_env() {
    #[derive(Config)]
    #[allow(dead_code)]
    #[config(layer_attr(derive(Debug, PartialEq)))]
    struct Conf {
        #[config(
            env = "PE_FUN",
            parse_env = confique::env::parse::list_by_comma,
            validate = validate_vec,
        )]
        function: Vec<u32>,

        #[config(
            env = "PE_AL",
            parse_env = confique::env::parse::list_by_colon,
            validate(assert_like.len() >= 3, "list too ~req short"),
        )]
        assert_like: Vec<u32>,

        #[config(
            env = "PE_FUN_OPT",
            parse_env = confique::env::parse::list_by_semicolon,
            validate = validate_vec,
        )]
        function_opt: Option<Vec<u32>>,

        #[config(
            env = "PE_AL_OPT",
            parse_env = confique::env::parse::list_by_space,
            validate(assert_like_opt.len() >= 3, "list too ~opt short"),
        )]
        assert_like_opt: Option<Vec<u32>>,
    }

    type Layer = <Conf as Config>::Layer;


    std::env::set_var("PE_FUN", "1,2");
    assert_err_contains(Layer::from_env(), "list too short");
    std::env::set_var("PE_FUN", "1,2,3");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        function: Some(vec![1, 2, 3]),
        assert_like: None,
        function_opt: None,
        assert_like_opt: None,
    });

    std::env::set_var("PE_AL", "1:2");
    assert_err_contains(Layer::from_env(), "list too ~req short");
    std::env::set_var("PE_AL", "1:2:3");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        function: Some(vec![1, 2, 3]),
        assert_like: Some(vec![1, 2, 3]),
        function_opt: None,
        assert_like_opt: None,
    });

    std::env::set_var("PE_FUN_OPT", "1;2");
    assert_err_contains(Layer::from_env(), "list too short");
    std::env::set_var("PE_FUN_OPT", "1;2;3");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        function: Some(vec![1, 2, 3]),
        assert_like: Some(vec![1, 2, 3]),
        function_opt: Some(vec![1, 2, 3]),
        assert_like_opt: None,
    });

    std::env::set_var("PE_AL_OPT", "1 2");
    assert_err_contains(Layer::from_env(), "list too ~opt short");
    std::env::set_var("PE_AL_OPT", "1 2 3");
    assert_eq!(Layer::from_env().unwrap(), Layer {
        function: Some(vec![1, 2, 3]),
        assert_like: Some(vec![1, 2, 3]),
        function_opt: Some(vec![1, 2, 3]),
        assert_like_opt: Some(vec![1, 2, 3]),
    });
}

#[test]
fn struct_validation() {
    #[derive(Config, PartialEq, Debug)]
    #[allow(dead_code)]
    #[config(validate = Self::validate)]
    struct Conf {
        foo: Option<u32>,
        bar: Option<u32>,
    }

    impl Conf {
        fn validate(&self) -> Result<(), &'static str> {
            if !(self.foo.is_some() ^ self.bar.is_some()) {
                return Err("exactly one of foo and bar must be set");
            }
            Ok(())
        }
    }

    let load = |s: &str| {
        let layer = toml::from_str::<<Conf as Config>::Layer>(s).unwrap();
        Conf::from_layer(layer)
    };

    assert_eq!(load("foo = 123").unwrap(), Conf {
        foo: Some(123),
        bar: None,
    });
    assert_eq!(load("bar = 27").unwrap(), Conf {
        foo: None,
        bar: Some(27),
    });
    assert_err_contains(load(""), "exactly one of foo and bar must be set");
    assert_err_contains(load("foo = 123\nbar=27"), "exactly one of foo and bar must be set");
}

#[track_caller]
fn assert_err_contains<T, E: std::fmt::Display>(r: Result<T, E>, expected: &str) {
    let e = r.map(|_| ()).unwrap_err();
    let s = format!("{e:#}");
    if !s.contains(expected) {
        panic!("expected error msg to contain '{expected}', but it doesn't: \n{s}");
    }
}
