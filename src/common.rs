//! Shared helpers: JS-compatible parsing and the negotiation skeleton.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;

/// The result of matching one provided option against one accepted spec: its specificity
/// `s`, the accepted spec's order `o`, and quality `q`.
pub(crate) struct Match {
    pub s: i32,
    pub o: i64,
    pub q: f64,
}

/// A scored provided option, carrying the index `pi` back into the `provided` slice.
pub(crate) struct Priority {
    pub s: i32,
    pub o: i64,
    pub q: f64,
    pub pi: usize,
}

/// Replicates the reference's priority update test
/// `(priority.s - spec.s || priority.q - spec.q || priority.o - spec.o) < 0`, where `||`
/// skips a zero or `NaN` term.
pub(crate) fn should_replace(prio: &Match, cand: &Match) -> bool {
    let ds = prio.s - cand.s;
    if ds != 0 {
        return ds < 0;
    }
    let dq = prio.q - cand.q;
    if dq != 0.0 && !dq.is_nan() {
        return dq < 0.0;
    }
    prio.o - cand.o < 0
}

/// Replicates `compareSpecs`: quality desc, specificity desc, order asc, index asc.
pub(crate) fn compare_priorities(a: &Priority, b: &Priority) -> Ordering {
    b.q.partial_cmp(&a.q)
        .unwrap_or(Ordering::Equal)
        .then_with(|| b.s.cmp(&a.s))
        .then_with(|| a.o.cmp(&b.o))
        .then_with(|| a.pi.cmp(&b.pi))
}

/// The shared negotiation skeleton used by charset, language, and media type.
///
/// * `accepts` — the parsed Accept-* header entries.
/// * `provided` — the server's available options, or `None` to return all acceptable
///   entries sorted by preference.
/// * `q_of` / `order_of` / `full` — accessors over an accept entry.
/// * `specify` — score one provided option against one accept entry.
pub(crate) fn negotiate<S>(
    accepts: &[S],
    provided: Option<&[&str]>,
    q_of: impl Fn(&S) -> f64,
    order_of: impl Fn(&S) -> i64,
    full: impl Fn(&S) -> String,
    specify: impl Fn(&str, &S) -> Option<Match>,
) -> Vec<String> {
    match provided {
        None => {
            let mut idx: Vec<usize> = (0..accepts.len())
                .filter(|&k| q_of(&accepts[k]) > 0.0)
                .collect();
            idx.sort_by(|&a, &b| {
                q_of(&accepts[b])
                    .partial_cmp(&q_of(&accepts[a]))
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| order_of(&accepts[a]).cmp(&order_of(&accepts[b])))
            });
            idx.into_iter().map(|k| full(&accepts[k])).collect()
        }
        Some(provided) => {
            let mut prios: Vec<Priority> = Vec::new();
            for (pi, &ptype) in provided.iter().enumerate() {
                let mut prio = Match {
                    s: 0,
                    o: -1,
                    q: 0.0,
                };
                for spec in accepts {
                    if let Some(cand) = specify(ptype, spec) {
                        if should_replace(&prio, &cand) {
                            prio = cand;
                        }
                    }
                }
                if prio.q > 0.0 {
                    prios.push(Priority {
                        s: prio.s,
                        o: prio.o,
                        q: prio.q,
                        pi,
                    });
                }
            }
            prios.sort_by(compare_priorities);
            prios
                .into_iter()
                .map(|p| provided[p.pi].to_string())
                .collect()
        }
    }
}

/// Whether `c` is matched by JavaScript's `\s` (and trimmed by `String.prototype.trim`).
pub(crate) fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// `String.prototype.trim()` — trim JS whitespace from both ends.
pub(crate) fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// Trim JS whitespace from the start only.
pub(crate) fn js_trim_start(s: &str) -> &str {
    s.trim_start_matches(is_js_whitespace)
}

/// Replicates JavaScript's `parseFloat`: trims leading whitespace, then reads the longest
/// leading decimal (or `Infinity`) literal; returns `NaN` if there is none.
pub(crate) fn parse_float(s: &str) -> f64 {
    let s = js_trim_start(s);
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // Optional sign.
    let negative = matches!(bytes.first(), Some(b'-'));
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        i = 1;
    }

    // Infinity.
    if s[i..].starts_with("Infinity") {
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // Mantissa: DIGITS, DIGITS '.' DIGITS?, or '.' DIGITS.
    let mut saw_digit = false;
    while i < len && bytes[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }
    if i < len && bytes[i] == b'.' {
        i += 1;
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return f64::NAN;
    }

    // Optional exponent — only consumed if it is well-formed.
    if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut k = i + 1;
        if k < len && (bytes[k] == b'+' || bytes[k] == b'-') {
            k += 1;
        }
        if k < len && bytes[k].is_ascii_digit() {
            while k < len && bytes[k].is_ascii_digit() {
                k += 1;
            }
            i = k;
        }
    }

    // `s[..i]` is a clean numeric literal; normalize the few forms Rust's parser rejects.
    normalize_and_parse(&s[..i])
}

/// Parse a JS-extracted numeric literal, fixing up `"5."`, `".5"`, trailing-dot, and a
/// lone leading `+` that Rust's `f64` parser may not accept directly.
fn normalize_and_parse(lit: &str) -> f64 {
    let lit = lit.strip_prefix('+').unwrap_or(lit);
    if let Ok(v) = lit.parse::<f64>() {
        return v;
    }
    // Insert a `0` for a leading/trailing dot and retry (e.g. ".5" -> "0.5", "5." -> "5.0").
    let mut fixed = String::with_capacity(lit.len() + 2);
    if let Some(rest) = lit.strip_prefix('.') {
        fixed.push_str("0.");
        fixed.push_str(rest);
    } else if let Some(rest) = lit.strip_prefix("-.") {
        fixed.push_str("-0.");
        fixed.push_str(rest);
    } else {
        fixed.push_str(lit);
    }
    if fixed.ends_with('.') {
        fixed.push('0');
    } else if let Some(pos) = fixed.find(['e', 'E']) {
        // A dot immediately before the exponent, e.g. "5.e2".
        if fixed.as_bytes().get(pos.wrapping_sub(1)) == Some(&b'.') {
            fixed.insert(pos, '0');
        }
    }
    fixed.parse::<f64>().unwrap_or(f64::NAN)
}

/// Matches `^\s*([^\s;]+)\s*(?:;(.*))?$` (the charset/encoding grammar): returns the token
/// and the optional parameter string after the first `;`, or `None` if it does not match.
pub(crate) fn match_simple(s: &str) -> Option<(&str, Option<&str>)> {
    let s = js_trim_start(s);
    let token_end = s
        .find(|c| is_js_whitespace(c) || c == ';')
        .unwrap_or(s.len());
    if token_end == 0 {
        return None;
    }
    let token = &s[..token_end];
    let rest = js_trim_start(&s[token_end..]);
    if rest.is_empty() {
        return Some((token, None));
    }
    rest.strip_prefix(';').map(|params| (token, Some(params)))
}

/// `[key, value?]` split on the first `'='` (`splitKeyValuePair`).
pub(crate) fn split_key_value(s: &str) -> (&str, Option<&str>) {
    match s.find('=') {
        None => (s, None),
        Some(i) => (&s[..i], Some(&s[i + 1..])),
    }
}

/// Number of `"` characters in `s`.
pub(crate) fn quote_count(s: &str) -> usize {
    s.bytes().filter(|&b| b == b'"').count()
}

/// Split `s` on `sep`, but re-join pieces that fall inside an open double-quote (matching
/// the reference's `splitMediaTypes` / `splitParameters` quote handling).
pub(crate) fn quote_aware_split(s: &str, sep: char) -> Vec<String> {
    let raw: Vec<&str> = s.split(sep).collect();
    let mut out: Vec<String> = Vec::new();
    for piece in raw {
        match out.last_mut() {
            Some(last) if quote_count(last) % 2 == 1 => {
                last.push(sep);
                last.push_str(piece);
            }
            _ => out.push(piece.to_string()),
        }
    }
    out
}
