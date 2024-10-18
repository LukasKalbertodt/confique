//! This example demonstrates the usage of validators for single fields or whole
//! structs. Try editing `files/validate.toml` to see different errors. Also
//! see the docs.

use std::time::Duration;

use confique::Config;


#[derive(Debug, Config)]
#[allow(dead_code)]
struct Conf {
    // Here, the validator is a function returning `Result<(), impl Display>`.
    #[config(validate = validate_name)]
    name: String,

    // For simple cases, validation can be written in this `assert!`-like style.
    #[config(env = "PORT", validate(*port >= 1024, "port must not require super-user"))]
    port: Option<u16>,

    #[config(nested)]
    watch: WatchConfig,
}

// You can also add validators for whole structs, which are called later in the
// pipeline, when all layers are already merged. These validators allow you to
// check fields in relationship to one another, e.g. maybe one field only makes
// sense to be set whenever another one has a specific value.
#[derive(Debug, Config)]
#[config(validate = Self::validate)]
struct WatchConfig {
    #[config(default = false)]
    busy_poll: bool,

    #[config(
        deserialize_with = deserialize_duration_ms,
        validate(*poll_period > Duration::from_millis(10), "cannot poll faster than 10ms"),
    )]
    poll_period: Option<Duration>,
}

fn validate_name(name: &String) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("name must be non-empty");
    }
    if !name.is_ascii() {
        return Err("name must be ASCII");
    }
    Ok(())
}

impl WatchConfig {
    fn validate(&self) -> Result<(), &'static str> {
        if !self.busy_poll && self.poll_period.is_some() {
            return Err("'poll_period' set, but busy polling is not enabled");
        }

        Ok(())
    }
}


pub(crate) fn deserialize_duration_ms<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let ms = <u64 as serde::Deserialize>::deserialize(deserializer)?;
    Ok(Duration::from_millis(ms))
}


fn main() {
    let r = Conf::builder()
        .env()
        .file("examples/files/validate.toml")
        .load();

    match r {
        Ok(conf) => println!("{:#?}", conf),
        Err(e) => println!("{e:#}"),
    }
}
