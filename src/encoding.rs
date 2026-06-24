//! `Accept-Encoding` negotiation (port of `lib/encoding.js`).
//!
//! This algorithm differs from the others: it injects an implicit `identity` encoding and
//! supports an optional `preferred` ordering, so it does not use the shared skeleton.

use crate::common::{js_trim, match_simple, parse_float, should_replace, split_key_value, Match};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;

struct EncSpec {
    encoding: String,
    q: f64,
    i: i64,
}

/// A unified item used for both the no-`provided` and `provided` sort paths. For the
/// no-`provided` path `s`/`o` are unset (0) so they tie, falling through to `idx`.
struct EncItem {
    encoding: String,
    q: f64,
    s: i32,
    o: i64,
    idx: i64,
}

fn parse_encoding(s: &str, i: i64) -> Option<EncSpec> {
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
    Some(EncSpec {
        encoding: token.to_string(),
        q,
        i,
    })
}

/// Whether `spec` matches `encoding` (exact case-insensitive, or `*`).
fn matches_encoding(encoding: &str, spec: &EncSpec) -> bool {
    spec.encoding.eq_ignore_ascii_case(encoding) || spec.encoding == "*"
}

fn specify(encoding: &str, spec: &EncSpec) -> Option<Match> {
    let s = if spec.encoding.eq_ignore_ascii_case(encoding) {
        1
    } else if spec.encoding != "*" {
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

fn parse_accept_encoding(accept: &str) -> Vec<EncSpec> {
    let parts: Vec<&str> = accept.split(',').collect();
    let original_len = parts.len() as i64;
    let mut accepts: Vec<EncSpec> = Vec::new();
    let mut has_identity = false;
    let mut min_quality = 1.0_f64;

    for (i, part) in parts.into_iter().enumerate() {
        if let Some(enc) = parse_encoding(js_trim(part), i as i64) {
            has_identity = has_identity || matches_encoding("identity", &enc);
            // `encoding.q || 1`: treat 0 / NaN as 1.
            let effective = if enc.q != 0.0 && !enc.q.is_nan() {
                enc.q
            } else {
                1.0
            };
            min_quality = min_quality.min(effective);
            accepts.push(enc);
        }
    }

    if !has_identity {
        accepts.push(EncSpec {
            encoding: "identity".to_string(),
            q: min_quality,
            i: original_len,
        });
    }

    accepts
}

fn encoding_priority(encoding: &str, accepts: &[EncSpec], pi: i64) -> EncItem {
    let mut prio = Match {
        s: 0,
        o: -1,
        q: 0.0,
    };
    for spec in accepts {
        if let Some(cand) = specify(encoding, spec) {
            if should_replace(&prio, &cand) {
                prio = cand;
            }
        }
    }
    EncItem {
        encoding: encoding.to_string(),
        q: prio.q,
        s: prio.s,
        o: prio.o,
        idx: pi,
    }
}

/// One comparator for both paths; with `preferred` it applies the preferred ordering on
/// quality ties, otherwise it is the standard `compareSpecs`.
fn compare(lhs: &EncItem, rhs: &EncItem, preferred: Option<&[&str]>) -> Ordering {
    let quality = rhs.q.partial_cmp(&lhs.q).unwrap_or(Ordering::Equal);
    if quality != Ordering::Equal {
        return quality;
    }
    let by_specs = || {
        rhs.s
            .cmp(&lhs.s)
            .then(lhs.o.cmp(&rhs.o))
            .then(lhs.idx.cmp(&rhs.idx))
    };
    if let Some(preferred) = preferred {
        let lhs_pref = preferred.iter().position(|&e| e == lhs.encoding);
        let rhs_pref = preferred.iter().position(|&e| e == rhs.encoding);
        return match (lhs_pref, rhs_pref) {
            (None, None) => by_specs(),
            (Some(li), Some(ri)) => li.cmp(&ri),
            (None, Some(_)) => Ordering::Greater, // preferred entries come first
            (Some(_), None) => Ordering::Less,
        };
    }
    by_specs()
}

pub(crate) fn preferred_encodings(
    accept: Option<&str>,
    provided: Option<&[&str]>,
    preferred: Option<&[&str]>,
) -> Vec<String> {
    // A missing or empty header leaves only the implicit `identity`.
    let header = accept.unwrap_or("");
    let accepts = parse_accept_encoding(header);

    match provided {
        None => {
            let mut items: Vec<EncItem> = accepts
                .iter()
                .filter(|s| s.q > 0.0)
                .map(|s| EncItem {
                    encoding: s.encoding.clone(),
                    q: s.q,
                    s: 0,
                    o: 0,
                    idx: s.i,
                })
                .collect();
            items.sort_by(|a, b| compare(a, b, preferred));
            items.into_iter().map(|item| item.encoding).collect()
        }
        Some(provided) => {
            let mut items: Vec<EncItem> = provided
                .iter()
                .enumerate()
                .map(|(pi, &enc)| encoding_priority(enc, &accepts, pi as i64))
                .filter(|item| item.q > 0.0)
                .collect();
            items.sort_by(|a, b| compare(a, b, preferred));
            items
                .into_iter()
                .map(|item| provided[item.idx as usize].to_string())
                .collect()
        }
    }
}
