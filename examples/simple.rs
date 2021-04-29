
mod config {
    use std::path::PathBuf;

    confique::config! {
        #![derive_for_all(Debug, Clone)]

        log: {
            /// Determines how many messages are logged. Log messages below
            /// this level are not emitted. Possible values: "trace", "debug",
            /// "info", "warn", "error" and "off".
            level: log::LevelFilter = "debug",

            /// If this is set, log messages are also written to this file.
            #[example = "/var/log/test.log"]
            file: Option<PathBuf>,
        },
    }
}


fn main() {

}
