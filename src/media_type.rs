//! `Accept` (media type) negotiation (port of `lib/mediaType.js`).

use crate::common::{
    is_js_whitespace, js_trim, js_trim_start, negotiate, parse_float, quote_aware_split,
    split_key_value, Match,
};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

struct MediaTypeSpec {
    type_: String,
    subtype: String,
    params: Vec<(String, String)>,
    q: f64,
    i: i64,
}

/// Matches `^\s*([^\s\/;]+)\/([^;\s]+)\s*(?:;(.*))?$`.
fn match_media_type(s: &str) -> Option<(&str, &str, Option<&str>)> {
    let s = js_trim_start(s);
    let type_end = s
        .find(|c| is_js_whitespace(c) || c == '/' || c == ';')
        .unwrap_or(s.len());
    if type_end == 0 {
        return None;
    }
    let type_ = &s[..type_end];
    let after_slash = s[type_end..].strip_prefix('/')?;
    let sub_end = after_slash
        .find(|c| c == ';' || is_js_whitespace(c))
        .unwrap_or(after_slash.len());
    if sub_end == 0 {
        return None;
    }
    let subtype = &after_slash[..sub_end];
    let rest = js_trim_start(&after_slash[sub_end..]);
    if rest.is_empty() {
        return Some((type_, subtype, None));
    }
    rest.strip_prefix(';')
        .map(|params| (type_, subtype, Some(params)))
}

/// Unwrap a single layer of surrounding double quotes (`splitKeyValuePair` + unquote).
fn unquote(val: Option<&str>) -> String {
    match val {
        Some(v) if v.starts_with('"') && v.ends_with('"') => {
            if v.len() >= 2 {
                v[1..v.len() - 1].to_string()
            } else {
                String::new() // a lone `"` slices to empty, like JS `slice(1, -1)`
            }
        }
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

fn parse_media_type(s: &str, i: i64) -> Option<MediaTypeSpec> {
    let (type_, subtype, params_str) = match_media_type(s)?;
    let mut params: Vec<(String, String)> = Vec::new();
    let mut q = 1.0;
    if let Some(params_str) = params_str {
        for kvp in quote_aware_split(params_str, ';') {
            let (key_raw, val) = split_key_value(js_trim(&kvp));
            let key = key_raw.to_ascii_lowercase();
            let value = unquote(val);
            if key == "q" {
                q = parse_float(&value);
                break;
            }
            params.push((key, value));
        }
    }
    Some(MediaTypeSpec {
        type_: type_.to_string(),
        subtype: subtype.to_string(),
        params,
        q,
        i,
    })
}

fn lookup<'a>(params: &'a [(String, String)], key: &str) -> &'a str {
    params
        .iter()
        .find(|(k, _)| k == key)
        .map_or("", |(_, v)| v.as_str())
}

fn specify(type_str: &str, spec: &MediaTypeSpec) -> Option<Match> {
    let p = parse_media_type(type_str, 0)?;
    let mut s = 0;

    if spec.type_.eq_ignore_ascii_case(&p.type_) {
        s |= 4;
    } else if spec.type_ != "*" {
        return None;
    }

    if spec.subtype.eq_ignore_ascii_case(&p.subtype) {
        s |= 2;
    } else if spec.subtype != "*" {
        return None;
    }

    if !spec.params.is_empty() {
        let all = spec
            .params
            .iter()
            .all(|(k, v)| v == "*" || v.eq_ignore_ascii_case(lookup(&p.params, k)));
        if all {
            s |= 1;
        } else {
            return None;
        }
    }

    Some(Match {
        s,
        o: spec.i,
        q: spec.q,
    })
}

fn parse_accept(accept: &str) -> Vec<MediaTypeSpec> {
    quote_aware_split(accept, ',')
        .iter()
        .enumerate()
        .filter_map(|(i, part)| parse_media_type(js_trim(part), i as i64))
        .collect()
}

pub(crate) fn preferred_media_types(
    accept: Option<&str>,
    provided: Option<&[&str]>,
) -> Vec<String> {
    // RFC 7231: a missing header means "*/*".
    let header = accept.unwrap_or("*/*");
    let accepts = parse_accept(header);
    negotiate(
        &accepts,
        provided,
        |s| s.q,
        |s| s.i,
        |s| {
            let mut full = String::with_capacity(s.type_.len() + 1 + s.subtype.len());
            full.push_str(&s.type_);
            full.push('/');
            full.push_str(&s.subtype);
            full
        },
        specify,
    )
}
