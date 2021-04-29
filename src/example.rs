//! This module demonstrate what a `config!` invocation will generate. This
//! module exists merely for documentation purposes and is not usable from your
//! crate.
//!
//! TODO

use std::{net::IpAddr, path::PathBuf};

// This is necessary because the macro generates a bunch of paths starting with
// `confique`, assuming that symbol is in scope.
#[doc(hidden)]
use crate as confique;

crate::config! {
    dns: {
        /// The DNS server IP address.
        #[example = "1.1.1.1"]
        server: IpAddr,

        /// Whether to use a local DNS resolution cache.
        use_cache: bool = true,

        /// How often to reattempt reaching the DNS server.
        retry_attempts: u32 = 27,
    },
    log: {
        /// Sets the log level. Possible values: "trace", "debug", "info",
        /// "warn", "error" and "off".
        level: log::LevelFilter = "info",

        /// If this is set, log messages are also written to this file.
        #[example = "/var/log/test.log"]
        file: Option<PathBuf>,
    },
}
