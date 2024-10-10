This is just a compile test to make sure `confique`'s derives also work when the using crate does not have `serde` in its direct dependencies.
Having this test as separate folder with a `run.rs` calling `cargo` was the easiest way I found to do that.
That way, this is also executed as part of `cargo test`.
