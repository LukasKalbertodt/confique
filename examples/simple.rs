use std::net::IpAddr;
use confique::{Config, Partial};

#[derive(Debug, Config)]
struct Http {
    #[config(default = 8080)]
    port: u16,

    #[config(default = "127.0.0.1")]
    bind: IpAddr,

    foo: Option<String>,
}


fn main() {
    println!("{:?}", Http::from_partial(<Http as Config>::Partial::default_values()));
}
