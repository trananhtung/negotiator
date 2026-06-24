//! # negotiator — HTTP content negotiation
//!
//! Choose the response representation a client prefers, from the `Accept`,
//! `Accept-Charset`, `Accept-Encoding`, and `Accept-Language` request headers. This is a
//! faithful Rust port of the widely-used [`negotiator`](https://www.npmjs.com/package/negotiator)
//! npm package (the engine behind Express's `req.accepts*`), which has no Rust equivalent.
//!
//! Each negotiation answers two questions:
//!
//! * Given the server's *available* options, which does the client prefer (and in what
//!   order)?
//! * Or, with no list provided, what does the client accept, sorted by preference?
//!
//! **Zero dependencies** and `#![no_std]` (needs only `alloc`).
//!
//! ```
//! use negotiator::Negotiator;
//!
//! let n = Negotiator::new()
//!     .accept("text/html, application/json;q=0.9, */*;q=0.1")
//!     .accept_language("en-US, fr;q=0.8")
//!     .accept_encoding("gzip, br;q=0.9");
//!
//! assert_eq!(n.media_type(Some(&["application/json", "text/html"])).as_deref(), Some("text/html"));
//! assert_eq!(n.language(Some(&["fr", "en"])).as_deref(), Some("en"));
//! assert_eq!(n.encoding(Some(&["gzip", "br"]), None).as_deref(), Some("gzip"));
//! ```
//!
//! The free functions [`preferred_media_types`], [`preferred_charsets`],
//! [`preferred_encodings`], and [`preferred_languages`] expose the same logic directly.
//! In all of them, a header of `None` means "absent" (which RFC 7231 treats as *accept
//! everything*), while `Some("")` means an explicit empty header (accept nothing).

#![no_std]
#![forbid(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/negotiator/0.1.0")]
// Indices here are small header/option positions; these i64<->usize conversions cannot
// realistically overflow or lose information.
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

mod charset;
mod common;
mod encoding;
mod language;
mod media_type;

// Compile-test the README's examples as part of `cargo test`.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// Return the acceptable media types from an `Accept` header, most-preferred first.
///
/// With `provided = Some(list)`, returns the subset of `list` the client accepts, ordered
/// by preference. With `provided = None`, returns every media type named in the header,
/// sorted. `accept = None` is treated as `*/*`.
///
/// ```
/// use negotiator::preferred_media_types;
/// let got = preferred_media_types(Some("text/html, application/json;q=0.9"), None);
/// assert_eq!(got, ["text/html", "application/json"]);
/// ```
#[must_use]
pub fn preferred_media_types(accept: Option<&str>, provided: Option<&[&str]>) -> Vec<String> {
    media_type::preferred_media_types(accept, provided)
}

/// Return the acceptable charsets from an `Accept-Charset` header, most-preferred first.
///
/// `accept = None` is treated as `*`.
#[must_use]
pub fn preferred_charsets(accept: Option<&str>, provided: Option<&[&str]>) -> Vec<String> {
    charset::preferred_charsets(accept, provided)
}

/// Return the acceptable encodings from an `Accept-Encoding` header, most-preferred first.
///
/// An implicit `identity` encoding is always considered (at the lowest advertised
/// quality) unless the header lists it or `*` explicitly. `preferred` biases ties toward a
/// server-preferred order. A missing or empty header leaves only `identity`.
#[must_use]
pub fn preferred_encodings(
    accept: Option<&str>,
    provided: Option<&[&str]>,
    preferred: Option<&[&str]>,
) -> Vec<String> {
    encoding::preferred_encodings(accept, provided, preferred)
}

/// Return the acceptable languages from an `Accept-Language` header, most-preferred first.
///
/// `accept = None` is treated as `*`.
#[must_use]
pub fn preferred_languages(accept: Option<&str>, provided: Option<&[&str]>) -> Vec<String> {
    language::preferred_languages(accept, provided)
}

/// A content negotiator over a request's `Accept*` headers.
///
/// Construct one with [`Negotiator::new`] and the chainable setters, then call the
/// negotiation methods. Each method takes `available = Some(list)` to pick from the
/// server's options, or `None` to list everything the client accepts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Negotiator<'a> {
    /// The `Accept` header value.
    pub accept: Option<&'a str>,
    /// The `Accept-Charset` header value.
    pub accept_charset: Option<&'a str>,
    /// The `Accept-Encoding` header value.
    pub accept_encoding: Option<&'a str>,
    /// The `Accept-Language` header value.
    pub accept_language: Option<&'a str>,
}

impl<'a> Negotiator<'a> {
    /// Create a negotiator with all headers absent.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the `Accept` header.
    #[must_use]
    pub fn accept(mut self, value: &'a str) -> Self {
        self.accept = Some(value);
        self
    }

    /// Set the `Accept-Charset` header.
    #[must_use]
    pub fn accept_charset(mut self, value: &'a str) -> Self {
        self.accept_charset = Some(value);
        self
    }

    /// Set the `Accept-Encoding` header.
    #[must_use]
    pub fn accept_encoding(mut self, value: &'a str) -> Self {
        self.accept_encoding = Some(value);
        self
    }

    /// Set the `Accept-Language` header.
    #[must_use]
    pub fn accept_language(mut self, value: &'a str) -> Self {
        self.accept_language = Some(value);
        self
    }

    /// All acceptable media types, or the preferred subset of `available`.
    #[must_use]
    pub fn media_types(&self, available: Option<&[&str]>) -> Vec<String> {
        media_type::preferred_media_types(self.accept, available)
    }

    /// The single most-preferred media type from `available`.
    #[must_use]
    pub fn media_type(&self, available: Option<&[&str]>) -> Option<String> {
        self.media_types(available).into_iter().next()
    }

    /// All acceptable charsets, or the preferred subset of `available`.
    #[must_use]
    pub fn charsets(&self, available: Option<&[&str]>) -> Vec<String> {
        charset::preferred_charsets(self.accept_charset, available)
    }

    /// The single most-preferred charset from `available`.
    #[must_use]
    pub fn charset(&self, available: Option<&[&str]>) -> Option<String> {
        self.charsets(available).into_iter().next()
    }

    /// All acceptable encodings, or the preferred subset of `available`.
    ///
    /// `preferred` biases quality ties toward a server-preferred order.
    #[must_use]
    pub fn encodings(&self, available: Option<&[&str]>, preferred: Option<&[&str]>) -> Vec<String> {
        encoding::preferred_encodings(self.accept_encoding, available, preferred)
    }

    /// The single most-preferred encoding from `available`.
    #[must_use]
    pub fn encoding(
        &self,
        available: Option<&[&str]>,
        preferred: Option<&[&str]>,
    ) -> Option<String> {
        self.encodings(available, preferred).into_iter().next()
    }

    /// All acceptable languages, or the preferred subset of `available`.
    #[must_use]
    pub fn languages(&self, available: Option<&[&str]>) -> Vec<String> {
        language::preferred_languages(self.accept_language, available)
    }

    /// The single most-preferred language from `available`.
    #[must_use]
    pub fn language(&self, available: Option<&[&str]>) -> Option<String> {
        self.languages(available).into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn media_type_basic() {
        assert_eq!(
            preferred_media_types(Some("text/html, application/json;q=0.9"), None),
            vec!["text/html", "application/json"]
        );
        assert_eq!(
            preferred_media_types(
                Some("text/*;q=0.5, text/html"),
                Some(&["text/html", "text/plain"])
            ),
            vec!["text/html", "text/plain"]
        );
    }

    #[test]
    fn media_type_wildcards_and_quality() {
        // q ordering: html (1) over json (0.9) over anything (0.1)
        let n = preferred_media_types(
            Some("*/*;q=0.1, application/json;q=0.9, text/html"),
            Some(&["image/png", "application/json", "text/html"]),
        );
        assert_eq!(n, vec!["text/html", "application/json", "image/png"]);
    }

    #[test]
    fn charset_default_is_star() {
        assert_eq!(
            preferred_charsets(None, Some(&["utf-8", "iso-8859-1"])),
            vec!["utf-8", "iso-8859-1"]
        );
        assert_eq!(
            preferred_charsets(
                Some("utf-8;q=0, iso-8859-1"),
                Some(&["utf-8", "iso-8859-1"])
            ),
            vec!["iso-8859-1"]
        );
    }

    #[test]
    fn encoding_injects_identity() {
        // identity is implicitly acceptable when absent from the header.
        assert_eq!(
            preferred_encodings(Some("gzip"), Some(&["identity", "gzip"]), None),
            vec!["gzip", "identity"]
        );
        // gzip not offered, only identity remains acceptable.
        assert_eq!(
            preferred_encodings(Some("gzip"), Some(&["identity"]), None),
            vec!["identity"]
        );
    }

    #[test]
    fn encoding_preferred_order_breaks_ties() {
        // gzip and br both q=1; `preferred` puts br first.
        assert_eq!(
            preferred_encodings(
                Some("gzip, br"),
                Some(&["gzip", "br"]),
                Some(&["br", "gzip"])
            ),
            vec!["br", "gzip"]
        );
    }

    #[test]
    fn language_prefix_matching() {
        assert_eq!(
            preferred_languages(Some("en-US, en;q=0.8"), Some(&["en", "en-US", "fr"])),
            vec!["en-US", "en"]
        );
        // a prefix request `en` matches a specific `en-US` offering
        assert_eq!(
            preferred_languages(Some("en"), Some(&["en-US"])),
            vec!["en-US"]
        );
    }

    #[test]
    fn negotiator_struct() {
        let n = Negotiator::new().accept("text/html").accept_language("fr");
        assert_eq!(
            n.media_type(Some(&["text/html", "text/plain"])).as_deref(),
            Some("text/html")
        );
        assert_eq!(n.language(Some(&["en", "fr"])).as_deref(), Some("fr"));
        assert_eq!(n.charset(Some(&["utf-8"])).as_deref(), Some("utf-8")); // no header → *
    }

    #[test]
    fn empty_header_accepts_nothing() {
        assert!(preferred_media_types(Some(""), Some(&["text/html"])).is_empty());
        assert!(preferred_charsets(Some(""), Some(&["utf-8"])).is_empty());
    }
}
