# Changelog

All notable changes to this project will be documented in this file.


## [Unreleased]


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


[Unreleased]: https://github.com/LukasKalbertodt/confique/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/LukasKalbertodt/confique/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/LukasKalbertodt/confique/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/LukasKalbertodt/confique/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/LukasKalbertodt/confique/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/LukasKalbertodt/confique/compare/v0.1.0...v0.1.1
