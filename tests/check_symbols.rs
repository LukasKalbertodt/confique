#![no_implicit_prelude]
#![allow(dead_code)]

extern crate confique;
extern crate std;

use confique::Config;


#[derive(Debug, Config)]
/// A sample configuration for our app.
struct Conf {
    #[config(nested)]
    http: Http,

    title: std::string::String,
}

/// Configuring the HTTP server of our app.
#[derive(Debug, Config)]
struct Http {
    /// The port the server will listen on.
    #[config(env = "PORT")]
    port: u16,

    /// The bind address of the server. Can be set to `0.0.0.0` for example, to
    /// allow other users of the network to access the server.
    #[config(default = "127.0.0.1")]
    bind: std::net::IpAddr,
}

#[test]
fn compiles() {}
