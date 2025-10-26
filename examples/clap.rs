//! This example demonstrate how to integrate with `clap` for CLI parsing
//! without duplicating code. The main trick is to derive `clap::Parser` for
//! the layer types and then include it in your main CLI definition via
//! `command(flatten)`.
//!
//! But of course it's also fine to have a separate type to separate CLI
//! definitions from configuration stuff. You can still load it by creating
//! layer types manually.

#![allow(dead_code)]

use std::{net::IpAddr, path::PathBuf};

use confique::Config;
use clap::Parser;


#[derive(Debug, Config)]
#[config(layer_attr(derive(clap::Args)))] // <--- derive clap::Args for layer
struct Conf {
    /// The port, duh.
    #[config(default = 8080)]
    #[config(layer_attr(arg(short, long)))] // <--- configure fields for clap
    port: u16,

    #[config(default = "127.0.0.1")]
    #[config(layer_attr(arg(long)))] // <--- configure fields for clap
    bind: IpAddr,
}

#[derive(Parser)]
struct Cli {
    /// Path to configuration file.
    #[arg(long, default_value = "examples/files/simple.toml")]
    config_file: PathBuf,

    #[command(flatten)] // <--- include layer type here and flatten it
    cli_config: <Conf as Config>::Layer,
}


fn main() {
    let cli = Cli::parse();

    let r = Conf::builder()
        .preloaded(cli.cli_config) // <--- Provide the preloaded layer to the builder
        .file(cli.config_file)
        .load();

    match r {
        Ok(conf) => println!("{:#?}", conf),
        Err(e) => println!("{e:#}"),
    }
}
