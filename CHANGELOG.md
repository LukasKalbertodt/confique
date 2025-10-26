# Changelog

All notable changes to this project will be documented in this file.


## [Unreleased]
- **Breaking**: rename `Partial` to `Layer` (including `from_partial` -> `from_layer`, `partial_attr` -> `layer_attr`, and more).
  - The new name more accurately reflects how the type is used most of the time. It is also less awkward to read/use as unlike "partial", "layer" is a proper noun.
- **Breaking**: require the (unresolved) symbol `Option` to be in scope and refer to `std::option::Option` where `derive(Config)` is used.
  - While technically breaking, this is unlikely to affect anyone, since hardly any Rust code shadows `Option`.
  - This change allows `confique` to emit just `Option` for the partial type, which in turn enables you to use other derives with it (e.g. `clap`).
- Allow multiple `validate` attributes on fields (all validations are performed).
- Forward field docs to layer type. This is especially useful when using derives on the layer, which read the docs.
- Add examples `clap` and `builder`, showing how you can derive useful traits for the layer type.
- Add clarification about `Config` vs `Deserialize` to the docs.
- (Technically breaking) Remove implicit crate feature `serde_yaml`. Use `yaml` instead!

## [0.3.1] - 2025-07-26
- Allow `#[config(partial_attr(...))]` on fields (thanks @aschey, in [#44](https://github.com/LukasKalbertodt/confique/pull/44))
- Update `toml` dependency to 0.9
- Fix outdated description in docs of `Partial::from_env`
- Fix typo in docs
- Fix license in `Cargo.toml` to use proper SPDX syntax
- Specify exact version requirement of dependencies

## [0.3.0] - 2024-10-18
- **Breaking**: Raise MSRV to 1.61.0
- **Breaking**: `toml`, `yaml` and `json5` are no longer default features ([`19d9ddc`](https://github.com/LukasKalbertodt/confique/commit/19d9ddc9537baf4e82274591ba92f02d4c5c1f36)). You now have to manually specify the features you need in `Cargo.toml`.
- **Breaking**: env vars set to an empty string, which fail to deserialize/parse/validate are now treated as not set. This is technically a breaking change, but I think this is the expected behavior and shouldn't affect you. ([#39](https://github.com/LukasKalbertodt/confique/pull/39))
- ⭐ Add validation feature ([#40](https://github.com/LukasKalbertodt/confique/pull/40))
  - `#[config(validate = path::to::function)] field: u32` to call the given function during field deserialization.
  - `#[config(validate(!s.is_empty(), "user must not be empty"))] user: String` is a `assert!`-style syntax to simple validation checks.
  - Validation can also be added to full structs.
  - See docs and examples for more information!
- Stop including `Error::source` in the `Display` output of `Error` ([`d454f0957`](https://github.com/LukasKalbertodt/confique/commit/d454f0957eb1cb4d566ebc448224b323a609d080))
- Improve & refactor docs of `derive(Config` a bit
- Update dependencies (syn to 2.0, heck to 0.5): this shouldn't affect you, except for faster compile times due to smaller dependency tree.


## [0.2.6] - 2024-10-10
- Fix compile errors when using `confique` derive without having `serde` in your direct dependencies (see [#38](https://github.com/LukasKalbertodt/confique/issues/38)).
- Update `toml` dependency to 0.8
- Fix some typos in docs

## [0.2.5] - 2023-12-10
- Add `#[config(partial_attr(...))]` struct attribute to specify attributes for
  the partial type.
- Allow "yes" and "no" as values when deserializing `bool` from env. Also, the
  match is done completely case insensitive now, such that e.g. "True", "tRuE"
  are accepted now.

## [0.2.4] - 2023-07-02
- Fixed enum deserialization from env values

## [0.2.3] - 2023-03-10
### Fixed
- Add `#[allow(missing_docs)]` to some generated code to avoid problems in
  crates that `#[forbid(missing_docs)]` globally.
- Fix badge in README

### Added
- Add short docs to generated module (to explains its purpose and avoid
  confusion when people find it in their docs)

### Changed
- Internal change that potentially improves compile time a tiny bit.

## [0.2.2] - 2022-11-25
### Fixed
- Use fully qualified paths for all symbols emitted by the derive macro.
  Before this, the derive would throw errors if you shadowed any of the symbols
  `Result`, `Option`, `Ok`, `None` or `Some`. A test has been added to make sure
  this does not happen again in the future.
  (Partially in [#23](https://github.com/LukasKalbertodt/confique/pull/23), thanks @aschey)


## [0.2.1] - 2022-11-06
### Added
- `parse_env` attribute for custom parsing of environment variables (allows you
  to load lists and other complex objects from env vars).
  (in [#22](https://github.com/LukasKalbertodt/confique/pull/22), thanks @cyphersnake)

### Changed
- Updated `serde_yaml` to 0.9 (this is only an internal dependency).

## [0.2.0] - 2022-10-21
### Added
- Add support for **array default values**, e.g. `#[config(default = [1, 2, 3])`
- Add support for **map default values**, e.g. `#[config(default = { "cat": 3, "dog": 5 })`
- **Add JSON5 support**
- Show environment variable key in config template
- Impl `PartialEq` for all `meta` items
- Impl `Serialize` for `meta::Expr`

### Changed
- **Breaking**: rename `{toml,yaml}::format` to `template`
- **Breaking**: make `FormatOptions` and some `meta` types `#[non_exhaustive]`
- Move to Rust 2021 (bumps MSRV to 1.56)
- Improved docs

### Fixed
- Fix type inference for float default values
- Fix name clash with generated helper functions
- Fix incorrect newlines for string default values in YAML config template

### Internal
- Rewrite large parts of the crate, mostly to deduplicate logic
- Add lots of tests

## [0.1.4] - 2022-10-14
### Fixed
- Derive attribute `env` can now be used together with `deserialize_with` (#2)

## [0.1.3] - 2022-04-07
### Fixed
- Derive macro does not product unparsable output anymore if the visibility
  modifier of the struct is `pub` or `pub(in path)`.

### Changed
- The output of `toml::format` now emits empty lines above nested objects in a
  more useful manner.


## [0.1.2] - 2022-03-30
### Fixed
- Fixed output of `toml::format` when leaf fields were listed after `nested`
  fields in a configuration.


## [0.1.1] - 2021-11-03
### Added
- `deserialize_with` attribute which is (basically) forwarded to `serde`

### Fixed
- Improve some spans in error messages


## 0.1.0 - 2021-07-28
### Added
- Everything.


[Unreleased]: https://github.com/LukasKalbertodt/confique/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/LukasKalbertodt/confique/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/LukasKalbertodt/confique/compare/v0.2.6...v0.3.0
[0.2.6]: https://github.com/LukasKalbertodt/confique/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/LukasKalbertodt/confique/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/LukasKalbertodt/confique/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/LukasKalbertodt/confique/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/LukasKalbertodt/confique/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/LukasKalbertodt/confique/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/LukasKalbertodt/confique/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/LukasKalbertodt/confique/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/LukasKalbertodt/confique/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/LukasKalbertodt/confique/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/LukasKalbertodt/confique/compare/v0.1.0...v0.1.1
