//! `Accept-Charset` negotiation (port of `lib/charset.js`).

use crate::common::{js_trim, match_simple, negotiate, parse_float, split_key_value, Match};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

struct CharsetSpec<'a> {
    charset: &'a str,
    q: f64,
    i: i64,
}

fn parse_accept_charset(accept: &str) -> Vec<CharsetSpec<'_>> {
    accept
        .split(',')
        .enumerate()
        .filter_map(|(i, part)| parse_charset(js_trim(part), i as i64))
        .collect()
}

fn parse_charset(s: &str, i: i64) -> Option<CharsetSpec<'_>> {
    let (token, params) = match_simple(s)?;
    let mut q = 1.0;
    if let Some(params) = params {
        for param in params.split(';') {
            let (key, value) = split_key_value(js_trim(param));
            if key == "q" {
                q = value.map_or(f64::NAN, parse_float);
                break;
            }
        }
    }
    Some(CharsetSpec {
        charset: token,
        q,
        i,
    })
}

/// `specify` for charsets: exact (case-insensitive) match scores `s = 1`, `*` scores 0.
fn specify(charset: &str, spec: &CharsetSpec<'_>) -> Option<Match> {
    let s = if spec.charset.eq_ignore_ascii_case(charset) {
        1
    } else if spec.charset != "*" {
        return None;
    } else {
        0
    };
    Some(Match {
        s,
        o: spec.i,
        q: spec.q,
    })
}

pub(crate) fn preferred_charsets(accept: Option<&str>, provided: Option<&[&str]>) -> Vec<String> {
    // RFC 7231: a missing header means "*".
    let header = accept.unwrap_or("*");
    let accepts = parse_accept_charset(header);
    negotiate(
        &accepts,
        provided,
        |s| s.q,
        |s| s.i,
        |s| s.charset.to_string(),
        specify,
    )
}
