//! `Accept-Language` negotiation (port of `lib/language.js`).

use crate::common::{is_js_whitespace, js_trim, js_trim_start, negotiate, parse_float, Match};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

struct LangSpec {
    prefix: String,
    full: String,
    q: f64,
    i: i64,
}

/// A language parsed for matching: its primary `prefix` and the `full` tag.
struct ParsedLang {
    prefix: String,
    full: String,
}

/// Matches `^\s*([^\s\-;]+)(?:-([^\s;]+))?\s*(?:;(.*))?$`.
fn match_language(s: &str) -> Option<(&str, Option<&str>, Option<&str>)> {
    let s = js_trim_start(s);
    let prefix_end = s
        .find(|c| is_js_whitespace(c) || c == '-' || c == ';')
        .unwrap_or(s.len());
    if prefix_end == 0 {
        return None;
    }
    let prefix = &s[..prefix_end];
    let mut rest = &s[prefix_end..];
    let mut suffix = None;
    if let Some(after_dash) = rest.strip_prefix('-') {
        let suffix_end = after_dash
            .find(|c| is_js_whitespace(c) || c == ';')
            .unwrap_or(after_dash.len());
        if suffix_end == 0 {
            return None; // a `-` not followed by a suffix character fails the whole match
        }
        suffix = Some(&after_dash[..suffix_end]);
        rest = &after_dash[suffix_end..];
    }
    let rest = js_trim_start(rest);
    if rest.is_empty() {
        return Some((prefix, suffix, None));
    }
    rest.strip_prefix(';')
        .map(|params| (prefix, suffix, Some(params)))
}

fn parsed_lang(s: &str) -> Option<ParsedLang> {
    let (prefix, suffix, _) = match_language(s)?;
    let full = match suffix {
        Some(suf) => {
            let mut f = String::with_capacity(prefix.len() + 1 + suf.len());
            f.push_str(prefix);
            f.push('-');
            f.push_str(suf);
            f
        }
        None => prefix.to_string(),
    };
    Some(ParsedLang {
        prefix: prefix.to_string(),
        full,
    })
}

fn parse_accept_language(accept: &str) -> Vec<LangSpec> {
    accept
        .split(',')
        .enumerate()
        .filter_map(|(i, part)| parse_language(js_trim(part), i as i64))
        .collect()
}

fn parse_language(s: &str, i: i64) -> Option<LangSpec> {
    let (prefix, suffix, params) = match_language(s)?;
    let full = match suffix {
        Some(suf) => {
            let mut f = String::with_capacity(prefix.len() + 1 + suf.len());
            f.push_str(prefix);
            f.push('-');
            f.push_str(suf);
            f
        }
        None => prefix.to_string(),
    };
    let mut q = 1.0;
    if let Some(params) = params {
        // Note: language params are NOT trimmed, and the LAST `q` wins (no early break).
        for param in params.split(';') {
            let mut kv = param.split('=');
            if kv.next() == Some("q") {
                q = kv.next().map_or(f64::NAN, parse_float);
            }
        }
    }
    Some(LangSpec {
        prefix: prefix.to_string(),
        full,
        q,
        i,
    })
}

fn specify(language: &str, spec: &LangSpec) -> Option<Match> {
    let p = parsed_lang(language)?;
    let s = if spec.full.eq_ignore_ascii_case(&p.full) {
        4
    } else if spec.prefix.eq_ignore_ascii_case(&p.full) {
        2
    } else if spec.full.eq_ignore_ascii_case(&p.prefix) {
        1
    } else if spec.full != "*" {
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

pub(crate) fn preferred_languages(accept: Option<&str>, provided: Option<&[&str]>) -> Vec<String> {
    // RFC 7231: a missing header means "*".
    let header = accept.unwrap_or("*");
    let accepts = parse_accept_language(header);
    negotiate(
        &accepts,
        provided,
        |s| s.q,
        |s| s.i,
        |s| s.full.clone(),
        specify,
    )
}
