//! Integration tests exercising the public API of `negotiator`.

use negotiator::{
    preferred_charsets, preferred_encodings, preferred_languages, preferred_media_types, Negotiator,
};

#[test]
fn browser_like_accept() {
    let accept = "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8";
    let n = Negotiator::new().accept(accept);
    assert_eq!(
        n.media_type(Some(&["application/json", "text/html"]))
            .as_deref(),
        Some("text/html")
    );
    // application/json only matches the */*;q=0.8 entry.
    assert_eq!(
        n.media_types(Some(&["application/json", "image/webp"])),
        ["image/webp", "application/json"]
    );
}

#[test]
fn quality_drives_order() {
    assert_eq!(
        preferred_media_types(Some("text/plain;q=0.5, text/html, text/x-dvi;q=0.8"), None),
        ["text/html", "text/x-dvi", "text/plain"]
    );
}

#[test]
fn charset_wildcard_and_explicit_zero() {
    assert_eq!(
        preferred_charsets(Some("utf-8;q=0, *"), Some(&["utf-8", "iso-8859-1"])),
        ["iso-8859-1"]
    );
}

#[test]
fn encoding_identity_and_preferred() {
    // identity is implicit; gzip wins when both are offered at equal quality.
    assert_eq!(
        preferred_encodings(
            Some("gzip, deflate"),
            Some(&["identity", "deflate", "gzip"]),
            None
        ),
        ["gzip", "deflate", "identity"]
    );
    // preferred biases the tie.
    assert_eq!(
        preferred_encodings(
            Some("gzip, deflate, br"),
            Some(&["gzip", "deflate", "br"]),
            Some(&["br", "deflate"])
        ),
        ["br", "deflate", "gzip"]
    );
}

#[test]
fn language_prefix_and_region() {
    assert_eq!(
        preferred_languages(Some("en-US, en;q=0.8"), Some(&["en", "en-US", "en-GB"])),
        ["en-US", "en", "en-GB"]
    );
}

#[test]
fn absent_headers_accept_everything() {
    let n = Negotiator::new();
    assert_eq!(
        n.media_type(Some(&["text/html"])).as_deref(),
        Some("text/html")
    );
    assert_eq!(n.charset(Some(&["utf-8"])).as_deref(), Some("utf-8"));
    assert_eq!(n.language(Some(&["fr"])).as_deref(), Some("fr"));
    // ...except encoding, where only identity is implicit.
    assert_eq!(
        n.encoding(Some(&["gzip", "identity"]), None).as_deref(),
        Some("identity")
    );
}
