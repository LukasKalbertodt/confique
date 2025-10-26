# Confique: type-safe, layered configuration library

[<img alt="CI status of main" src="https://img.shields.io/github/actions/workflow/status/LukasKalbertodt/confique/ci.yaml?branch=main&label=CI&logo=github&logoColor=white&style=for-the-badge" height="23">](https://github.com/LukasKalbertodt/confique/actions/workflows/ci.yaml)
[<img alt="Crates.io Version" src="https://img.shields.io/crates/v/confique?logo=rust&style=for-the-badge" height="23">](https://crates.io/crates/confique)
[<img alt="docs.rs" src="https://img.shields.io/crates/v/confique?color=blue&label=docs&style=for-the-badge" height="23">](https://docs.rs/confique)

Confique is a rather light-weight library that helps with configuration management in a type-safe and DRY (don't repeat yourself) fashion.

**Features**:

- **Type safe**: the code using the config values does not need to parse strings or `unwrap` any `Option`s.
  All values already have the correct type.
- **Layered configuration**: you can load from and then merge multiple sources of configuration.
- **Load config values from**:
    - Environment variables
    - Files: [TOML](https://toml.io/), [YAML](https://yaml.org/), and [JSON5](https://json5.org/)
    - Anything with a `serde` Deserializer
- **Based on `serde`**: less code in `confique` (more light-weight) and access to a huge ecosystem of high quality parsers.
- **Easily generate configuration "templates"**: describe all available config values to your users without repeating yourself.
- **Simple validation**: validity checks can easily be added via attributes.


## Simple example

```rust
use std::{net::IpAddr, path::PathBuf};
use confique::Config;


#[derive(Config)]
struct Conf {
    /// Port to listen on.
    #[config(env = "PORT", default = 8080)]
    port: u16,

    /// Bind address.
    #[config(default = "127.0.0.1")]
    address: IpAddr,

    #[config(nested)]
    log: LogConf,
}

#[derive(Config)]
struct LogConf {
    #[config(default = true)]
    stdout: bool,

    #[config(validate(file.is_absolute(), "log file requires absolute path"))]
    file: Option<PathBuf>,

    #[config(default = ["debug"])]
    ignored_modules: Vec<String>,
}


let config = Conf::builder()
    .env()
    .file("example-app.toml")
    .file("/etc/example-app/config.toml")
    .load()?;
```

See [**the documentation**](https://docs.rs/confique) for more information.

### Configuration Template

With the above example, you can automatically generate a configuration template:
a file in a chosen format that lists all values with their description, default values, and env values.

<table>
<tr>
    <td><code>toml::template::&lt;Conf&gt;()</code></td>
    <td><code>yaml::template::&lt;Conf&gt;()</code></td>
    <td><code>json5::template::&lt;Conf&gt;()</code></td>
</tr>
<tr>
<td>

```toml
# Port to listen on.
#
# Can also be specified via
# environment variable `PORT`.
#
# Default value: 8080
#port = 8080

# Bind address.
#
# Default value: "127.0.0.1"
#address = "127.0.0.1"

[log]
# <omitted>
```

</td>
<td>

```yaml
# Port to listen on.
#
# Can also be specified via
# environment variable `PORT`.
#
# Default value: 8080
#port: 8080

# Bind address.
#
# Default value: 127.0.0.1
#address: 127.0.0.1

log:
  # <omitted>
```

</td>
<td>

```json5
{
  // Port to listen on.
  //
  // Can also be specified via
  // environment variable `PORT`.
  //
  // Default value: 8080
  //port: 8080,

  // Bind address.
  //
  // Default value: "127.0.0.1"
  //address: "127.0.0.1",

  log: {
    // <omitted>
  },
}
```

</td>
</tr>
</table>

<sup>(Note: The "environment variable" sentence is on a single line; I just split it into two lines for readability in this README.)</sup>

## Comparison with other libraries/solutions

### [`config`](https://crates.io/crates/config)

- Loosely typed:
    - You access configuration values via string path (e.g. `"http.port"`) and deserialize at "use site".
    - No defined schema
- More features
- Larger library
- If you need a "config template", you need to repeat code/docs

### [`figment`](https://crates.io/crates/figment)

- Also based on `serde` and also uses your own structs as data store, thus type safe
- Instead of using layer types, aggregates different layers in a dynamic data store
- If you need a "config template", you need to repeat code/docs

### Just `serde`?

Serde is not a configuration, but a deserialization library.
But you can get surprisingly far with just serde and it might actually be sufficient for your project.
However, once you want to load from multiple sources, you either have make all your fields `Option` or repeat code/docs.
With `confique` you also get some other handy helpers.


## Status of this project

There is still some design space to explore and there are certainly still many features one could add.
However, the core interface (the derive macro and the core traits) probably won't change a lot anymore.
Confique is used by a web project (that's already used in production) which I'm developing alongside of confique.


<br />

---

## License

Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
