# Changelog

All notable changes to this project will be documented in this file.


## [Unreleased]


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


[Unreleased]: https://github.com/LukasKalbertodt/confique/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/LukasKalbertodt/confique/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/LukasKalbertodt/confique/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/LukasKalbertodt/confique/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/LukasKalbertodt/confique/compare/v0.1.0...v0.1.1
