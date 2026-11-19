use std::{collections::HashMap, net::IpAddr, path::PathBuf};

use pretty_assertions::assert_eq;

use confique::{layer, Config, Layer};


#[derive(Debug, Config)]
#[config(layer_attr(derive(Debug)))]
#[allow(dead_code)]
struct Conf {
    name: String,

    #[config(default = 8080)]
    port: u16,

    log_file: Option<PathBuf>,

    #[config(nested)]
    http: Http,

    #[config(nested)]
    misc: Misc,
}

#[derive(Debug, Config)]
#[config(layer_attr(derive(Debug)))]
#[allow(dead_code)]
struct Http {
    #[config(default = "127.0.0.1")]
    bind: IpAddr,

    headers: Vec<String>,

    #[config(validate(*timeout <= 60, "timeout too long"))]
    timeout: Option<u32>,

    #[config(nested)]
    tls: Tls,
}

#[derive(Debug, Config)]
#[config(layer_attr(derive(Debug)))]
#[allow(dead_code)]
struct Tls {
    cert: Option<PathBuf>,
    enabled: Option<bool>,
}

#[derive(Debug, Config)]
#[config(layer_attr(derive(Debug)))]
#[allow(dead_code)]
struct Misc {
    weights: Option<HashMap<String, f32>>,
    rgb: Option<[u8; 3]>,
    big: Option<u64>,
    ratio: Option<f64>,

    matrix: Option<Vec<Vec<u8>>>,
    groups: Option<HashMap<String, Vec<String>>>,
    rows: Option<Vec<HashMap<String, u32>>>,
    deep: Option<HashMap<String, HashMap<String, u32>>>,

    #[config(deserialize_with = deserialize_shout)]
    shout: Option<String>,

    #[config(validate = validate_r_type)]
    r#type: Option<String>,
}

fn deserialize_shout<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <String as serde::Deserialize>::deserialize(deserializer)?;
    Ok(s.to_uppercase())
}

fn validate_r_type(v: &String) -> Result<(), &'static str> {
    if v.is_empty() {
        Err("must not be empty")
    } else {
        Ok(())
    }
}


#[test]
fn empty() {
    let layer = layer!(Conf {}).unwrap();
    assert!(layer.is_empty());
}

#[test]
fn leaf_values() {
    let layer = layer!(Conf {
        name: "peter",
        port: 1234,
        log_file: "/tmp/app.log",
    }).unwrap();

    assert_eq!(layer.name, Some("peter".into()));
    assert_eq!(layer.port, Some(1234));
    assert_eq!(layer.log_file, Some("/tmp/app.log".into()));
    assert!(layer.http.is_empty());
    assert!(layer.misc.is_empty());
}

#[test]
fn nested() {
    let layer = layer!(Conf {
        http: {
            bind: "::1",
            tls: { enabled: true },
        },
        name: "peter",
    }).unwrap();

    assert_eq!(layer.name, Some("peter".into()));
    assert_eq!(layer.port, None);
    assert_eq!(layer.http.bind, Some(IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1])));
    assert_eq!(layer.http.headers, None);
    assert_eq!(layer.http.tls.cert, None);
    assert_eq!(layer.http.tls.enabled, Some(true));
}

#[test]
fn nested_empty() {
    let layer = layer!(Conf { name: "peter", http: {} }).unwrap();
    assert_eq!(layer.name, Some("peter".into()));
    assert!(layer.http.is_empty());
}

#[test]
fn arrays_and_maps() {
    let layer = layer!(Conf {
        http: { headers: ["x-user", "x-password",] },
        misc: {
            weights: { "a": 1.5, "b": 2.0, },
            rgb: [255u8, 0, 127],
        },
    }).unwrap();

    assert_eq!(layer.http.headers, Some(vec!["x-user".into(), "x-password".into()]));
    assert_eq!(
        layer.misc.weights,
        Some(HashMap::from([("a".into(), 1.5), ("b".into(), 2.0)])),
    );
    assert_eq!(layer.misc.rgb, Some([255, 0, 127]));

    let layer = layer!(Conf {
        http: { headers: [] },
        misc: { weights: {} },
    }).unwrap();
    assert_eq!(layer.http.headers, Some(vec![]));
    assert_eq!(layer.misc.weights, Some(HashMap::new()));
}

#[test]
fn nested_arrays_and_maps() {
    let peter = "peter";
    let layer = layer!(Conf {
        misc: {
            matrix: [[1, 2, 3], [], [4]],
            groups: { "admins": [peter, "anna"], "guests": ["bob"] },
            rows: [{ "a": 1 }, {}, { "b": 2, "c": 3 }],
            deep: { "x": { "y": 1 }, "z": {} },
        },
    }).unwrap();

    let strings = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let map = |v: &[(&str, u32)]| v.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    assert_eq!(layer.misc.matrix, Some(vec![vec![1, 2, 3], vec![], vec![4]]));
    assert_eq!(layer.misc.groups, Some(HashMap::from([
        ("admins".into(), strings(&["peter", "anna"])),
        ("guests".into(), strings(&["bob"])),
    ])));
    assert_eq!(
        layer.misc.rows,
        Some(vec![map(&[("a", 1)]), map(&[]), map(&[("b", 2), ("c", 3)])]),
    );
    assert_eq!(layer.misc.deep, Some(HashMap::from([
        ("x".into(), map(&[("y", 1)])),
        ("z".into(), map(&[])),
    ])));
}

#[test]
fn nested_empty_arrays_and_maps() {
    let layer = layer!(Conf {
        misc: {
            matrix: [[], []],
            groups: { "admins": [], "guests": [] },
            rows: [{}, {}],
            deep: { "x": {} },
        },
    }).unwrap();

    assert_eq!(layer.misc.matrix, Some(vec![vec![], vec![]]));
    assert_eq!(layer.misc.groups, Some(HashMap::from([
        ("admins".into(), vec![]),
        ("guests".into(), vec![]),
    ])));
    assert_eq!(layer.misc.rows, Some(vec![HashMap::new(), HashMap::new()]));
    assert_eq!(layer.misc.deep, Some(HashMap::from([("x".into(), HashMap::new())])));
}

#[test]
fn numbers() {
    let layer = layer!(Conf {
        misc: {
            big: 5_000_000_000u64,
            ratio: 3,
        },
    }).unwrap();

    assert_eq!(layer.misc.big, Some(5_000_000_000));
    assert_eq!(layer.misc.ratio, Some(3.0));
}

#[test]
fn variables() {
    let name = String::from("peter");
    let headers = vec!["a", "b"];
    let weights = HashMap::from([("x", 1.0f32)]);
    let layer = layer!(Conf {
        name: name.as_str(),
        http: { headers: headers.clone() },
        misc: { weights: weights },
    }).unwrap();

    assert_eq!(layer.name, Some("peter".into()));
    assert_eq!(layer.http.headers, Some(vec!["a".into(), "b".into()]));
    assert_eq!(layer.misc.weights, Some(HashMap::from([("x".into(), 1.0)])));
}

#[test]
fn rust_expressions() {
    let users = vec!["peter", "anna"];
    let other = vec![1u8];
    let x = 7u32;
    let layer = layer!(Conf {
        // Not arrays/maps, as something follows the `[...]` or as the `{...}`
        // does not start with a literal key.
        name: ["a", "b"].concat(),
        log_file: { let dir = "/tmp"; dir },
        http: {
            headers: users.iter().map(|s| s.to_uppercase()).collect::<Vec<_>>(),
        },
        misc: {
            // Rust expressions next to array/map syntax.
            matrix: [[1], other.clone(), [], vec![3; 2]],
            groups: { "admins": users, "guests": ["bob"], "none": [] },
            rows: [{ "a": x + 1, "b": x }, { "c": 3 }],
            ratio: if x > 3 { 1.5 } else { 2.5 },
            rgb: [1u8; 3],
        },
    }).unwrap();

    assert_eq!(layer.name, Some("ab".into()));
    assert_eq!(layer.log_file, Some("/tmp".into()));
    assert_eq!(layer.http.headers, Some(vec!["PETER".into(), "ANNA".into()]));
    assert_eq!(layer.misc.matrix, Some(vec![vec![1], vec![1], vec![], vec![3, 3]]));
    assert_eq!(layer.misc.groups.unwrap()["admins"], ["peter", "anna"]);
    assert_eq!(layer.misc.rows, Some(vec![
        HashMap::from([("a".into(), 8), ("b".into(), 7)]),
        HashMap::from([("c".into(), 3)]),
    ]));
    assert_eq!(layer.misc.ratio, Some(1.5));
    assert_eq!(layer.misc.rgb, Some([1, 1, 1]));
}

/// Commas inside of generic arguments and closure parameters must not be
/// treated as the end of the value. And `<` and `|` are also operators.
#[test]
fn commas_in_expressions() {
    let (a, b) = (1u32, 2u32);
    let layer = layer!(Conf {
        port: <HashMap<u8, u16>>::from([(1, 2)])[&1] + ((a << b) | b) as u16,
        http: {
            headers: [
                ["a", "b"].iter().fold(String::new(), |acc, s| acc + s),
                (|x: &str, y: &str| -> String { format!("{x}{y}") })("c", "d"),
            ],
        },
        misc: {
            weights: [("a", 1.5f32), ("b", 2.0)].into_iter().collect::<HashMap<_, _>>(),
            matrix: [
                <Vec<u8> as From<[u8; 2]>>::from([1, 2]),
                [(a < b) as u8, (a > b) as u8],
                if a < b { vec![3] } else { vec![] },
                [4, 5].iter().map(|x: &u8| -> u8 { *x }).collect::<Vec<u8>>(),
                Triple::<u8, [u8; 2], u8>::new().0,
            ],
            groups: { "a": Vec::<&str>::new(), "b": <Vec<&str>>::new() },
            big: 1 + <u64 as From<u32>>::from(a << b) + (a as Alias<u8, u8> <= 3) as u64,
            ratio: -<f64 as From<f32>>::from(1.0),
            deep: {
                "x": [("y", 1)].into_iter().collect::<HashMap<_, _>>(),
                "z": HashMap::<&str, u32>::new(),
            },
        },
    }).unwrap();

    assert_eq!(layer.port, Some(8));
    assert_eq!(layer.http.headers, Some(vec!["ab".into(), "cd".into()]));
    assert_eq!(layer.misc.weights, Some(HashMap::from([("a".into(), 1.5), ("b".into(), 2.0)])));
    assert_eq!(
        layer.misc.matrix,
        Some(vec![vec![1, 2], vec![1, 0], vec![3], vec![4, 5], vec![]]),
    );
    assert_eq!(layer.misc.groups, Some(HashMap::from([
        ("a".into(), vec![]),
        ("b".into(), vec![]),
    ])));
    assert_eq!(layer.misc.big, Some(6));
    assert_eq!(layer.misc.ratio, Some(-1.0));
    assert_eq!(layer.misc.deep, Some(HashMap::from([
        ("x".into(), HashMap::from([("y".into(), 1)])),
        ("z".into(), HashMap::new()),
    ])));
}

type Alias<A, B> = <(A, B) as First>::Out;

trait First {
    type Out;
}

impl<A, B> First for (A, B) {
    type Out = u32;
}

struct Triple<A, B, C>(Vec<A>, std::marker::PhantomData<(B, C)>);

impl<A, B, C> Triple<A, B, C> {
    fn new() -> Self {
        Self(vec![], std::marker::PhantomData)
    }
}

#[test]
fn raw_values() {
    let path = PathBuf::from("/tmp/app.log");
    let layer = layer!(Conf {
        port: #(42),
        log_file: #(path.clone()),
        http: {
            bind: #(IpAddr::from([10, 0, 0, 1])),
            timeout: #(1000), // validators are not run for raw values
        },
    }).unwrap();

    assert_eq!(layer.port, Some(42));
    assert_eq!(layer.log_file, Some(path));
    assert_eq!(layer.http.bind, Some(IpAddr::from([10, 0, 0, 1])));
    assert_eq!(layer.http.timeout, Some(1000));
    assert_eq!(
        layer.validate().unwrap_err().to_string(),
        "validation failed for 'http.timeout': timeout too long",
    );
}

#[test]
fn raw_nested() {
    let http = layer!(Http { bind: "10.0.0.1" }).unwrap();
    let layer = layer!(Conf { port: 42, http: #(http) }).unwrap();

    assert_eq!(layer.port, Some(42));
    assert_eq!(layer.http.bind, Some(IpAddr::from([10, 0, 0, 1])));
    assert_eq!(layer.http.timeout, None);
}

#[test]
fn question_mark_in_values() {
    fn make(port: &str, timeout: &str) -> Result<ConfLayer, Box<dyn std::error::Error>> {
        Ok(layer!(Conf {
            port: #(port.parse()?),
            http: { timeout: timeout.parse::<u32>()? },
        })?)
    }

    type ConfLayer = <Conf as Config>::Layer;
    assert_eq!(make("80", "3").unwrap().port, Some(80));
    assert_eq!(make("80", "3").unwrap().http.timeout, Some(3));
    assert!(make("foo", "3").is_err());
    assert!(make("80", "foo").is_err());
}

/// The variables used by the generated code must not be visible to the
/// expressions passed to the macro.
#[test]
fn hygiene() {
    let layer = 1;
    let err = 2;
    let path = "foo";
    let value = "bar";
    let out = layer!(Conf {
        port: layer + err,
        name: [path, value].concat(),
        log_file: #(path.into()),
        http: { timeout: #(layer) },
    }).unwrap();

    assert_eq!(out.port, Some(3));
    assert_eq!(out.name, Some("foobar".into()));
    assert_eq!(out.log_file, Some("foo".into()));
    assert_eq!(out.http.timeout, Some(1));
}

#[test]
fn inside_layer_macro() {
    let layer = layer!(Conf {
        name: layer!(Tls { cert: "foo" }).unwrap().cert.unwrap().to_str().unwrap(),
    }).unwrap();
    assert_eq!(layer.name, Some("foo".into()));
}

#[test]
fn deserialize_with() {
    let layer = layer!(Conf { misc: { shout: "hello" } }).unwrap();
    assert_eq!(layer.misc.shout, Some("HELLO".into()));
}

#[test]
fn raw_ident() {
    let layer = layer!(Conf { misc: { r#type: "fast" } }).unwrap();
    assert_eq!(layer.misc.r#type, Some("fast".into()));

    let err = layer!(Conf { misc: { r#type: "" } }).unwrap_err();
    assert_eq!(
        format!("{err:#}"),
        "failed to deserialize configuration from field 'misc.type': \
            validation failed for 'type': must not be empty",
    );
}

#[test]
fn errors() {
    let err = layer!(Conf { http: { timeout: 100 } }).unwrap_err();
    assert_eq!(
        format!("{err:#}"),
        "failed to deserialize configuration from field 'http.timeout': \
            validation failed for 'timeout': timeout too long",
    );

    // The rest of the message is from serde.
    let err = layer!(Conf { port: 70000 }).unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.starts_with("failed to deserialize configuration from field 'port': "),
        "unexpected error message: {msg}",
    );

    let err = layer!(Conf { http: { bind: "localhost" } }).unwrap_err();
    assert_eq!(
        err.to_string(),
        "failed to deserialize configuration from field 'http.bind'",
    );

    // Values after the first error are not evaluated.
    let mut evaluated = false;
    let res = layer!(Conf { port: -1, name: { evaluated = true; "peter" } });
    assert!(res.is_err());
    assert!(!evaluated);
}

#[test]
fn with_defaults() {
    let layer = layer!(Conf {
        name: "peter",
        http: { headers: [] },
    }).unwrap();

    let conf = Conf::builder().preloaded(layer).load().unwrap();
    assert_eq!(conf.name, "peter");
    assert_eq!(conf.port, 8080);
    assert_eq!(conf.http.bind, IpAddr::from([127, 0, 0, 1]));
    assert_eq!(conf.http.headers, Vec::<String>::new());
    assert_eq!(conf.log_file, None);
}

#[test]
fn config_macro() {
    let conf = confique::config!(Conf {
        name: "peter",
        http: { headers: ["x-user"] },
    }).unwrap();
    assert_eq!(conf.name, "peter");
    assert_eq!(conf.port, 8080);
    assert_eq!(conf.http.bind, IpAddr::from([127, 0, 0, 1]));
    assert_eq!(conf.http.headers, vec!["x-user".to_string()]);

    let err = confique::config!(Conf { http: { headers: [] } }).unwrap_err();
    assert_eq!(err.to_string(), "required configuration value is missing: 'name'");

    let err = confique::config!(Conf { name: "peter", port: -1 }).unwrap_err();
    assert_eq!(err.to_string(), "failed to deserialize configuration from field 'port'");
}
