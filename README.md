# Confique: type-safe, layered configuration library

[<img alt="CI status of main" src="https://img.shields.io/github/workflow/status/LukasKalbertodt/confique/CI/main?label=CI&logo=github&logoColor=white&style=for-the-badge" height="23">](https://github.com/LukasKalbertodt/confique/actions?query=workflow%3ACI+branch%3Amaster)
[<img alt="Crates.io Version" src="https://img.shields.io/crates/v/confique?logo=rust&style=for-the-badge" height="23">](https://crates.io/crates/confique)
[<img alt="docs.rs" src="https://img.shields.io/crates/v/confique?color=blue&label=docs&style=for-the-badge" height="23">](https://docs.rs/confique)

Confique is a rather light-weight library that helps with configuration management in a type-safe and DRY (don't repeat yourself) fashion.

**Features**:

- **Type safe**: the code using the config values does not need to parse strings or `unwrap` any `Option`s.
  All values already have the correct type.
- **Layered configuration**: you can load from and then merge multiple sources of configuration.
- **Load config values from**:
    - Environment variables
    - Files: TOML & YAML
    - Anything with a `serde` Deserializer (built-in support for more formats coming soon)
- **Based on `serde`**: less code in `confique` (more light-weight) and access to a huge ecosystem of high quality parsers.
- Easily generate configuration "templates" to describe all available config values to your users.


## Simple example

```rust
use std::path::PathBuf;
use confique::Config;


#[derive(Config)]
struct Conf {
    #[config(env = "EXAMPLE_APP_USERNAME")]
    username: String,

    #[config(env = "EXAMPLE_APP_BUFFER_SIZE", default = 4096)]
    buffer_size: u32,

    #[config(nested)]
    log: LogConf,
}

#[derive(Config)]
struct LogConf {
    #[config(default = true)]
    stdout: bool,

    file: Option<PathBuf>,
}


let config = Conf::builder()
    .env()
    .file("example-app.toml")
    .file("/etc/example-app/config.toml")
    .load()?;
```

See [**the documentation**](https://docs.rs/confique) for more information.

## Status of this project

Confique is still a very young project.
There are lots of features and improvements already planned.
I'm developing this library alongside a web project that uses it.


<br />

---

## License

Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
