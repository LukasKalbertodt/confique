use std::{net::IpAddr, time::Duration};
use confique::Config;

/// The root configuration.
#[derive(Config)]
pub struct Conf {
    /// The main color of the app.
    pub color: Option<String>,

    /// HTTP server configuration.
    #[config(nested)]
    pub http: HttpConf,
}

/// Configuration for the HTTP server.
#[derive(Config)]
#[config(layer_attr(derive(Debug, Clone)))]
pub struct HttpConf {
    /// The port the server will listen on.
    #[config(env = "APP_PORT", validate(*port >= 1024, "port must be >= 1024"))]
    pub port: u16,

    /// The address the server will bind to.
    #[config(default = "127.0.0.1")]
    pub bind: IpAddr,

    /// Additional headers that are passed through.
    #[config(default = ["x-user", "x-password"])]
    pub headers: Vec<String>,

    /// Timeout for requests, in seconds.
    #[config(default = 30, deserialize_with = duration_from_secs)]
    pub timeout: Duration,
}

/// Deserializes a `Duration` from a number of seconds.
pub fn duration_from_secs<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: confique::serde::Deserializer<'de>,
{
    let secs = <u64 as confique::serde::Deserialize>::deserialize(deserializer)?;
    Ok(Duration::from_secs(secs))
}
