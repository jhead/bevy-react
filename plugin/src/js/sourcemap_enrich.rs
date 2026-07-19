//! Source-map enrichment for JS stacks.
//!
//! Boa VM frames often look like `at name (url:line:column)` when modules were
//! loaded with a path (Vite HTTP URLs). This module:
//! - caches decoded maps per canonical URL
//! - strips Vite HMR query/hash from URLs
//! - honours `//# sourceMappingURL=` (relative, absolute, or `data:`)
//! - falls back to `{url}.map`
//!
//! Remaining gap: frames that are only bare function names (no location) cannot
//! be symbolicated. See docs/TROUBLESHOOTING.md.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Mutex, OnceLock};

use sourcemap::SourceMap;

static MAP_CACHE: OnceLock<Mutex<HashMap<String, Option<CachedMap>>>> = OnceLock::new();

#[derive(Clone)]
struct CachedMap {
    sm: SourceMap,
}

fn map_cache() -> &'static Mutex<HashMap<String, Option<CachedMap>>> {
    MAP_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Attempt to rewrite stack frames that include `url:line:column` via source maps.
pub fn enrich_stack_with_sourcemaps(stack: Option<String>) -> Option<String> {
    let stack = stack?;
    if stack.is_empty() {
        return Some(stack);
    }

    let mut changed = false;
    let mut out = String::with_capacity(stack.len());
    for (i, line) in stack.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if let Some(mapped) = try_map_frame_line(line) {
            out.push_str(&mapped);
            changed = true;
        } else {
            out.push_str(line);
        }
    }

    if changed {
        Some(out)
    } else {
        Some(stack)
    }
}

fn try_map_frame_line(line: &str) -> Option<String> {
    let (prefix, url, line_1, col_1, suffix) = parse_frame_location(line)?;
    let sm = load_sourcemap_cached(url)?;
    // Source maps use 0-based line/column; stacks are 1-based.
    let token = sm
        .sm
        .lookup_token(line_1.saturating_sub(1), col_1.saturating_sub(1))?;
    let src = token.get_source().unwrap_or(url);
    let src_line = token.get_src_line() + 1;
    let src_col = token.get_src_col() + 1;
    let name = token.get_name();

    let mapped = match name {
        Some(n) => format!("{src}:{src_line}:{src_col} ({n})"),
        None => format!("{src}:{src_line}:{src_col}"),
    };
    Some(format!("{prefix}{mapped}{suffix}"))
}

/// Parse `…(url:line:col)…` or `…url:line:col…` from a stack line.
fn parse_frame_location(line: &str) -> Option<(&str, &str, u32, u32, &str)> {
    // Prefer parenthesized V8 / Boa style: at foo (http://…:10:5)
    if let Some(open) = line.rfind('(') {
        if let Some(close) = line[open + 1..].find(')') {
            let inner = &line[open + 1..open + 1 + close];
            if let Some((url, line_1, col_1)) = split_url_line_col(inner) {
                return Some((&line[..=open], url, line_1, col_1, &line[open + 1 + close..]));
            }
        }
    }

    // SpiderMonkey / bare: foo@http://…:10:5  or  http://…:10:5
    let start = find_url_start(line)?;
    let rest = &line[start..];
    let end = rest.find([' ', '\t', ')']).unwrap_or(rest.len());
    let loc = &rest[..end];
    let (url, line_1, col_1) = split_url_line_col(loc)?;
    Some((&line[..start], url, line_1, col_1, &rest[end..]))
}

fn find_url_start(line: &str) -> Option<usize> {
    // Prefer well-formed schemes; also accept Path-collapsed `http:/host`.
    [
        "https://",
        "http://",
        "file://",
        "https:/",
        "http:/",
        "file:/",
    ]
    .into_iter()
    .filter_map(|scheme| line.find(scheme))
    .min()
}

fn split_url_line_col(s: &str) -> Option<(&str, u32, u32)> {
    let col_sep = s.rfind(':')?;
    let col: u32 = s[col_sep + 1..].parse().ok()?;
    let line_sep = s[..col_sep].rfind(':')?;
    let line: u32 = s[line_sep + 1..col_sep].parse().ok()?;
    let url = &s[..line_sep];
    if url.is_empty() {
        return None;
    }
    // Avoid treating `C:\…` or time-like strings as locations.
    if !url.contains('/') && !url.contains('\\') {
        return None;
    }
    Some((url, line, col))
}

/// Canonicalize URL for cache / fetch: fix `http:/` → `http://`, drop `?` / `#`.
fn canonicalize_url(url: &str) -> String {
    let mut u = url.to_string();
    for (bad, good) in [
        ("https:/", "https://"),
        ("http:/", "http://"),
        ("file:/", "file://"),
    ] {
        if u.starts_with(bad) && !u.starts_with(good) {
            u = format!("{good}{}", &u[bad.len()..]);
            break;
        }
    }
    if let Some(i) = u.find(['?', '#']) {
        u.truncate(i);
    }
    u
}

fn load_sourcemap_cached(url: &str) -> Option<CachedMap> {
    let key = canonicalize_url(url);
    {
        let cache = map_cache().lock().ok()?;
        if let Some(entry) = cache.get(&key) {
            return entry.clone();
        }
    }

    let loaded = load_sourcemap_uncached(&key).map(|sm| CachedMap { sm });

    if let Ok(mut cache) = map_cache().lock() {
        cache.insert(key, loaded.clone());
    }
    loaded
}

fn load_sourcemap_uncached(url: &str) -> Option<SourceMap> {
    // 1) Fetch the script and honour sourceMappingURL when present.
    if let Some(map_bytes) = load_map_via_source_comment(url) {
        if let Ok(sm) = SourceMap::from_reader(Cursor::new(&map_bytes)) {
            return Some(sm);
        }
    }

    // 2) Conventional adjacent `.map`.
    let map_url = format!("{url}.map");
    let map_bytes = fetch_bytes(&map_url)?;
    SourceMap::from_reader(Cursor::new(map_bytes)).ok()
}

fn load_map_via_source_comment(script_url: &str) -> Option<Vec<u8>> {
    let source = fetch_text(script_url)?;
    let map_ref = parse_source_mapping_url(&source)?;
    resolve_map_reference(script_url, &map_ref)
}

/// Last `//# sourceMappingURL=` or `//@ sourceMappingURL=` in the file.
fn parse_source_mapping_url(source: &str) -> Option<String> {
    let mut found = None;
    for line in source.lines().rev() {
        let trimmed = line.trim();
        let value = trimmed
            .strip_prefix("//# sourceMappingURL=")
            .or_else(|| trimmed.strip_prefix("//@ sourceMappingURL="))
            .or_else(|| {
                // Block-comment form used by some bundlers.
                trimmed
                    .strip_prefix("/*# sourceMappingURL=")
                    .and_then(|s| s.strip_suffix("*/"))
                    .map(str::trim)
            })?;
        let value = value.trim();
        if !value.is_empty() {
            found = Some(value.to_string());
            break;
        }
    }
    found
}

fn resolve_map_reference(script_url: &str, map_ref: &str) -> Option<Vec<u8>> {
    if let Some(data) = map_ref.strip_prefix("data:") {
        return decode_data_url(data);
    }

    let map_url = if map_ref.starts_with("http://")
        || map_ref.starts_with("https://")
        || map_ref.starts_with("file://")
    {
        map_ref.to_string()
    } else {
        // Relative to the script URL directory.
        join_url(script_url, map_ref)?
    };
    fetch_bytes(&map_url)
}

fn join_url(base: &str, rel: &str) -> Option<String> {
    if let Ok(base_url) = url::Url::parse(base) {
        return base_url.join(rel).ok().map(|u| u.to_string());
    }
    // file paths / odd bases
    let parent = base.rsplit_once('/')?.0;
    Some(format!("{parent}/{rel}"))
}

fn decode_data_url(data: &str) -> Option<Vec<u8>> {
    // data:[<mediatype>][;base64],<data>
    let (meta, payload) = data.split_once(',')?;
    if meta.split(';').any(|p| p.eq_ignore_ascii_case("base64")) {
        decode_base64(payload.trim())
    } else {
        Some(percent_decode(payload))
    }
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if bytes.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            break;
        }
        let a = val(bytes[i])?;
        let b = val(*bytes.get(i + 1)?)?;
        out.push((a << 2) | (b >> 4));
        let c_byte = *bytes.get(i + 2)?;
        if c_byte == b'=' {
            break;
        }
        let c = val(c_byte)?;
        out.push(((b & 0xf) << 4) | (c >> 2));
        let d_byte = *bytes.get(i + 3)?;
        if d_byte == b'=' {
            break;
        }
        let d = val(d_byte)?;
        out.push(((c & 0x3) << 6) | d);
        i += 4;
    }
    Some(out)
}

fn percent_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    if let Some(path) = url.strip_prefix("file://") {
        return std::fs::read(path).ok();
    }

    #[cfg(all(feature = "fetch", not(target_arch = "wasm32")))]
    if url.starts_with("http://") || url.starts_with("https://") {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .ok()?;
        let resp = client.get(url).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        return resp.bytes().ok().map(|b| b.to_vec());
    }

    let _ = url;
    None
}

fn fetch_text(url: &str) -> Option<String> {
    let bytes = fetch_bytes(url)?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn split_url_line_col_parses_http() {
        let (url, line, col) =
            split_url_line_col("http://localhost:5173/src/App.tsx:12:3").unwrap();
        assert_eq!(url, "http://localhost:5173/src/App.tsx");
        assert_eq!(line, 12);
        assert_eq!(col, 3);
    }

    #[test]
    fn parse_v8_style_frame() {
        let line = "    at App (http://localhost:5173/src/App.tsx:12:3)";
        let (prefix, url, line_1, col_1, suffix) = parse_frame_location(line).unwrap();
        assert_eq!(prefix, "    at App (");
        assert_eq!(url, "http://localhost:5173/src/App.tsx");
        assert_eq!(line_1, 12);
        assert_eq!(col_1, 3);
        assert_eq!(suffix, ")");
    }

    #[test]
    fn parse_collapsed_http_slash() {
        let line = "    at App (http:/localhost:5173/src/App.tsx:12:3)";
        let (_prefix, url, line_1, col_1, _suffix) = parse_frame_location(line).unwrap();
        assert_eq!(url, "http:/localhost:5173/src/App.tsx");
        assert_eq!(line_1, 12);
        assert_eq!(col_1, 3);
        assert_eq!(
            canonicalize_url(url),
            "http://localhost:5173/src/App.tsx"
        );
    }

    #[test]
    fn canonicalize_strips_hmr_query() {
        assert_eq!(
            canonicalize_url("http://localhost:5173/src/App.tsx?t=123#x"),
            "http://localhost:5173/src/App.tsx"
        );
    }

    #[test]
    fn parse_source_mapping_url_last_wins() {
        let src = "//# sourceMappingURL=old.map\ncode();\n//# sourceMappingURL=App.tsx.map\n";
        assert_eq!(
            parse_source_mapping_url(src).as_deref(),
            Some("App.tsx.map")
        );
    }

    #[test]
    fn decode_data_url_base64() {
        // {"version":3} base64
        let raw = decode_data_url(
            "application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozfQ==",
        )
        .unwrap();
        assert_eq!(String::from_utf8(raw).unwrap(), "{\"version\":3}");
    }

    #[test]
    fn enrich_noop_without_maps() {
        let stack = Some("Error: boom\n    at mystery".to_string());
        let out = enrich_stack_with_sourcemaps(stack.clone());
        assert_eq!(out, stack);
    }

    #[test]
    fn enrich_file_map_rewrites_frame() {
        let dir = std::env::temp_dir().join(format!(
            "bevy-react-sourcemap-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let js_path = dir.join("bundle.js");
        let map_path = dir.join("bundle.js.map");

        // Minimal map: generated (0,0) → original.ts
        let map = r#"{
  "version": 3,
  "file": "bundle.js",
  "sources": ["original.ts"],
  "names": ["boom"],
  "mappings": "AAAA"
}"#;
        std::fs::write(&map_path, map).unwrap();
        let mut js = std::fs::File::create(&js_path).unwrap();
        writeln!(js, "throw new Error('x');").unwrap();
        writeln!(js, "//# sourceMappingURL=bundle.js.map").unwrap();

        let file_url = format!("file://{}", js_path.display());
        // Clear any prior cache entry for this URL.
        if let Ok(mut cache) = map_cache().lock() {
            cache.remove(&canonicalize_url(&file_url));
        }

        let stack = format!("Error: x\n    at boom ({file_url}:1:1)");
        let out = enrich_stack_with_sourcemaps(Some(stack)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            out.contains("original.ts"),
            "expected original.ts in enriched stack, got:\n{out}"
        );
    }
}
