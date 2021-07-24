use std::{net::IpAddr, path::Path};
use confique::Config;

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


fn main() -> Result<(), anyhow::Error> {
    println!("{:#?}", Conf::META);

    let r = Conf::from_sources(&[
        &Path::new("examples/files/simple.toml"),
        &Path::new("examples/files/etc/simple.yaml"),
    ])?;

    println!("{:#?}", r);

    Ok(())
}
