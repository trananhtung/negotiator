# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0]

### Added

- Initial release: a faithful, zero-dependency, `no_std` port of the `negotiator` npm
  package (v1).
- `Negotiator` over the four `Accept*` headers, plus the free functions
  `preferred_media_types`, `preferred_charsets`, `preferred_encodings`, and
  `preferred_languages`.
- Quality values, wildcards, media-type parameters, language prefix matching, implicit
  `identity` encoding, and the optional encoding `preferred` ordering.
