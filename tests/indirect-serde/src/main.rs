#![allow(dead_code)]

use confique::Config;

#[derive(Config)]
struct Conf {
    #[config(deserialize_with = my_deserialize_fn)]
    username: String,

    normal: u32,

    opt: Option<String>,

    #[config(nested)]
    nested: Nested,
}

#[derive(Config)]
struct Nested {
    #[config(env = "APP_PORT")]
    port: u16,

    #[config(default = "127.0.0.1")]
    bind: std::net::IpAddr,

    #[config(default = ["x-user", "x-password"])]
    headers: Vec<String>,
}

fn my_deserialize_fn<'de, D>(_: D) -> Result<String, D::Error>
where
    D: confique::serde::Deserializer<'de>,
{
    todo!()
}

fn main() {}
