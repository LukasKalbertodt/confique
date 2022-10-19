pub(crate) mod example1;
pub(crate) mod example2;


#[allow(unused_macros)]
macro_rules! include_format_output {
    ($file:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/format-output/", $file))
    };
}

#[allow(unused_imports)]
pub(crate) use include_format_output;
