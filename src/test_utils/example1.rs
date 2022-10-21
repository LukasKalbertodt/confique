use std::{
    net::IpAddr,
    path::PathBuf,
};

use crate::Config;
use crate as confique;

#[derive(Debug, Config)]
/// A sample configuration for our app.
pub struct Conf {
    /// Name of the website.
    pub site_name: String,

    /// Configurations related to the HTTP communication.
    #[config(nested)]
    pub http: Http,

    /// Configuring the logging.
    #[config(nested)]
    pub log: LogConfig,
}

/// Configuring the HTTP server of our app.
#[derive(Debug, Config)]
pub struct Http {
    /// The port the server will listen on.
    #[config(env = "PORT")]
    pub port: u16,

    #[config(nested)]
    pub headers: Headers,

    /// The bind address of the server. Can be set to `0.0.0.0` for example, to
    /// allow other users of the network to access the server.
    #[config(default = "127.0.0.1")]
    pub bind: IpAddr,
}

#[derive(Debug, Config)]
pub struct Headers {
    /// The header in which the reverse proxy specifies the username.
    #[config(default = "x-username")]
    pub username: String,

    /// The header in which the reverse proxy specifies the display name.
    #[config(default = "x-display-name")]
    pub display_name: String,

    /// Headers that are allowed.
    #[config(default = ["content-type", "content-encoding"])]
    pub allowed: Vec<String>,
}


#[derive(Debug, Config)]
pub struct LogConfig {
    /// If set to `true`, the app will log to stdout.
    #[config(default = true)]
    pub stdout: bool,

    /// If this is set, the app will write logs to the given file. Of course,
    /// the app has to have write access to that file.
    pub file: Option<PathBuf>,
}
