use std::net::IpAddr;
use confique::{Config, Partial};

#[derive(Debug, Config)]
struct Conf {
    #[config(child)]
    http: Http,

    #[config(child)]
    cat: Cat,
}


#[derive(Debug, Config)]
struct Http {
    #[config(default = 8080)]
    port: u16,

    #[config(default = "127.0.0.1")]
    bind: IpAddr,
}

#[derive(Debug, Config)]
struct Cat {
    foo: Option<String>,
}


fn main() {
    println!("{:#?}", Conf::from_partial(<Conf as Config>::Partial::default_values()));
}
