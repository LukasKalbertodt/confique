use std::{net::IpAddr, path::PathBuf};
use confique::{Config, toml::FormatOptions};

#[derive(Debug, Config)]
/// A sample configuration for our app.
struct Conf {
    #[config(nested)]
    http: Http,

    #[config(nested)]
    log: LogConfig,
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
    bind: IpAddr,
}

#[derive(Debug, Config)]
struct LogConfig {
    /// If set to `true`, the app will log to stdout.
    #[config(default = true)]
    stdout: bool,

    /// If this is set, the app will write logs to the given file. Of course,
    /// the app has to have write access to that file.
    file: Option<PathBuf>,
}


fn main() -> Result<(), anyhow::Error> {
    println!("TEMPLATE:");
    println!("--------------------------------------------------------");
    print!("{}", confique::toml::format::<Conf>(FormatOptions::default()));
    println!("--------------------------------------------------------");

    let r = Conf::builder()
        .file("examples/files/simple.toml")
        .file("examples/files/etc/simple.yaml")
        .load()?;

    println!();
    println!("LOADED CONFIGURATION: {:#?}", r);

    Ok(())
}
