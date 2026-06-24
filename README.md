# negotiator

[![crates.io](https://img.shields.io/crates/v/negotiator.svg)](https://crates.io/crates/negotiator)
[![docs.rs](https://docs.rs/negotiator/badge.svg)](https://docs.rs/negotiator)
[![CI](https://github.com/trananhtung/negotiator/actions/workflows/ci.yml/badge.svg)](https://github.com/trananhtung/negotiator/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/negotiator.svg)](#license)

**HTTP content negotiation for Rust.**

Pick the response representation a client prefers from the `Accept`, `Accept-Charset`,
`Accept-Encoding`, and `Accept-Language` request headers — quality values, wildcards,
language prefix matching, and all.

`negotiator` is a faithful Rust port of the widely-used
[`negotiator`](https://www.npmjs.com/package/negotiator) npm package (the engine behind
Express's `req.accepts*`), which has no Rust equivalent.

- **Zero dependencies**
- **`#![no_std]`** (needs only `alloc`)
- Differential-tested against the reference `negotiator` implementation across all four
  negotiations

## Install

```toml
[dependencies]
negotiator = "0.1"
```

## Usage

```rust
use negotiator::Negotiator;

let n = Negotiator::new()
    .accept("text/html, application/json;q=0.9, */*;q=0.1")
    .accept_language("en-US, fr;q=0.8")
    .accept_encoding("gzip, br;q=0.9");

// Pick the best of what the server can produce.
assert_eq!(n.media_type(Some(&["application/json", "text/html"])).as_deref(), Some("text/html"));
assert_eq!(n.language(Some(&["fr", "en"])).as_deref(), Some("en"));
assert_eq!(n.encoding(Some(&["gzip", "br"]), None).as_deref(), Some("gzip"));

// Or list everything the client accepts, in order of preference.
assert_eq!(
    n.media_types(None),
    ["text/html", "application/json", "*/*"]
);
```

You can also call the free functions directly:

```rust
use negotiator::preferred_languages;

assert_eq!(
    preferred_languages(Some("en-US, en;q=0.9, fr;q=0.8"), Some(&["en", "fr"])),
    ["en", "fr"]
);
```

## Semantics

Each negotiation parses the relevant header into quality-weighted entries and ranks the
server's `available` options by **quality**, then **specificity** (an exact match beats a
wildcard; `en-US` beats `en` beats `*`), then the header and option order — exactly as the
npm package does.

- A header argument of `None` means the header is **absent**, which RFC 7231 treats as
  *accept everything* (`*` / `*/*`). `Some("")` means an explicit empty header (accept
  nothing). `Accept-Encoding` is the exception: a missing or empty header leaves only the
  implicit `identity` encoding.
- `preferred_encodings` always considers an implicit `identity` encoding (at the lowest
  advertised quality) unless the header lists `identity` or `*`, and accepts an optional
  `preferred` list to bias quality ties toward a server-preferred order.

## Note on case folding

The reference uses JavaScript's `toLowerCase`; this crate folds case with ASCII rules,
which is identical for the ASCII tokens that appear in these headers (media types,
charsets, encodings, language tags) and differs only on exotic non-ASCII input.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
