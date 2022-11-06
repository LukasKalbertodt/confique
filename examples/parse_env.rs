#![allow(dead_code)]

use confique::{
    env_utils::{
        to_collection_by_char_separator, to_collection_by_comma, to_collection_by_semicolon,
    },
    Config,
};
use std::{collections::HashSet, num::NonZeroU64, path::PathBuf, str::FromStr, convert::Infallible};

#[derive(Debug, Config)]
struct Conf {
    #[config(
        env = "PATHS",
        parse_env = to_collection_by_comma
    )]
    paths: HashSet<PathBuf>,
    #[config(
         env = "PORTS",
         parse_env = to_collection_by_semicolon
    )]
    ports: Vec<u16>,
    #[config(
         env = "NAMES",
         parse_env = to_collection_by_char_separator::<'|', _, _>
    )]
    names: Vec<String>,
    #[config(
         env = "TIMEOUT",
         parse_env = NonZeroU64::from_str,
    )]
    timeout_seconds: NonZeroU64,
    #[config(
         env = "FORMATS",
         parse_env = parse_formats,
    )]
    formats: Vec<Format>,
}

#[derive(Debug, serde::Deserialize)]
enum Format {
    Env,
    Toml,
    Json5,
    Yaml,
}

fn parse_formats(input: &str) -> Result<Vec<Format>, Infallible> {
    let mut result = Vec::new();

    if input.contains("toml") {
        result.push(Format::Toml);
    }
    if input.contains("env") {
        result.push(Format::Env);
    }
    if input.contains("yaml") {
        result.push(Format::Yaml);
    }
    if input.contains("json5") {
        result.push(Format::Json5);
    }

    Ok(result)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PATHS", "/bin/ls,/usr/local/bin,/usr/bin/ls");
    std::env::set_var("PORTS", "8080;8888;8000");
    std::env::set_var(
        "NAMES",
        "Mervinc Harmon|Alfreda Valenzuela|Arlen Cabrera|Damon Rice|Willie Schwartz",
    );
    std::env::set_var("TIMEOUT", "100");
    std::env::set_var("FORMATS", "json5,yaml;.env");

    println!("{:#?}", Conf::builder().env().load()?);

    Ok(())
}
