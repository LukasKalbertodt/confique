//! Example config with all fields having defaults - for serialization tests.

use std::path::PathBuf;

use crate as confique;
use crate::Config;

/// A sample configuration where all fields have defaults.
#[derive(Debug, Clone, PartialEq, Config)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Conf {
    /// Name of the app.
    #[config(default = "my-app")]
    pub name: String,

    /// Server configuration.
    #[config(nested)]
    pub server: Server,

    /// Logging configuration.
    #[config(nested)]
    pub log: LogConfig,
}

/// Server settings.
#[derive(Debug, Clone, PartialEq, Config)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Server {
    /// The port to listen on.
    #[config(default = 8080)]
    pub port: u16,

    /// The host to bind to.
    #[config(default = "localhost")]
    pub host: String,

    /// Enable TLS.
    #[config(default = false)]
    pub tls: bool,
}

/// Logging settings.
#[derive(Debug, Clone, PartialEq, Config)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct LogConfig {
    /// Log to stdout.
    #[config(default = true)]
    pub stdout: bool,

    /// Optional log file path.
    pub file: Option<PathBuf>,
}

impl Conf {
    /// Create a config with all default values.
    pub fn with_defaults() -> Self {
        Self {
            name: "my-app".to_string(),
            server: Server {
                port: 8080,
                host: "localhost".to_string(),
                tls: false,
            },
            log: LogConfig {
                stdout: true,
                file: None,
            },
        }
    }
}
