pub(crate) mod example1;


macro_rules! include_format_output {
    ($file:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/format-output/", $file))
    };
}

pub(crate) use include_format_output;
