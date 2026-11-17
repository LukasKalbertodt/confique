#![no_implicit_prelude]
#![allow(dead_code)]

extern crate confique;
extern crate std;

use confique::Config;

// This one symbol is required
use std::option::Option;


#[derive(Debug, Config)]
/// A sample configuration for our app.
#[config(validate = Self::validate)]
struct Conf {
    #[config(nested)]
    http: Http,

    #[config(nested)]
    misc: Misc,

    #[config(deserialize_with = deserialize_title, layer_attr(serde(alias = "name")))]
    title: std::string::String,
}

impl Conf {
    fn validate(&self) -> std::result::Result<(), &'static str> {
        std::result::Result::Ok(())
    }
}

/// Configuring the HTTP server of our app.
#[derive(Debug, Config)]
#[config(layer_attr(derive(Debug)))]
struct Http {
    /// The port the server will listen on.
    #[config(env = "PORT", validate = validate_port)]
    port: u16,

    /// The bind address of the server. Can be set to `0.0.0.0` for example, to
    /// allow other users of the network to access the server.
    #[config(default = "127.0.0.1")]
    bind: std::net::IpAddr,

    #[config(validate(!user.is_empty(), "user must not be empty"))]
    user: Option<std::string::String>,
}

#[derive(Debug, Config)]
struct Misc {
    #[config(env = "CHECK_SYMBOLS_LIST", parse_env = confique::env::parse::list_by_comma)]
    list: std::vec::Vec<u32>,

    #[config(
        env = "CHECK_SYMBOLS_VALIDATED_LIST",
        parse_env = confique::env::parse::list_by_comma,
        validate(validated_list.len() > 1, "too short"),
    )]
    validated_list: std::vec::Vec<u32>,

    #[config(default = [])]
    empty_array: std::vec::Vec<u32>,

    #[config(default = {})]
    empty_map: std::collections::HashMap<std::string::String, u32>,

    #[config(default = { "a": 1 })]
    map: std::collections::HashMap<std::string::String, u32>,
}

fn validate_port(port: &u16) -> std::result::Result<(), &'static str> {
    if *port == 0 {
        return std::result::Result::Err("port must not be 0");
    }
    std::result::Result::Ok(())
}

fn deserialize_title<'de, D>(deserializer: D) -> std::result::Result<std::string::String, D::Error>
where
    D: confique::serde::Deserializer<'de>,
{
    <std::string::String as confique::serde::Deserialize>::deserialize(deserializer)
}

#[test]
fn compiles() {}
