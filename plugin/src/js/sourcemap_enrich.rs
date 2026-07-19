//! Best-effort source-map enrichment for JS stacks.
//!
//! Boa stacks are often VM frame names without `file:line:column`, and Vite
//! maps are not always reachable from the native host. When a frame *does*
//! look like `url:line:col`, we try to load `{url}.map` (file:// or http(s)
//! when the `fetch` feature is enabled) and rewrite that frame.
//!
//! TODO: cache maps per URL; honour `//# sourceMappingURL=` (including data:);
//! symbolicate full Vite HMR stacks once Boa emits browser-like frames.

use std::io::Cursor;

use sourcemap::SourceMap;

/// Attempt to rewrite stack frames that include `url:line:column` via adjacent `.map` files.
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
    let map_bytes = load_map_for_url(url)?;
    let sm = SourceMap::from_reader(Cursor::new(map_bytes)).ok()?;
    // Source maps use 0-based line/column; stacks are 1-based.
    let token = sm.lookup_token(line_1.saturating_sub(1), col_1.saturating_sub(1))?;
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
    // Prefer parenthesized V8-style: at foo (http://…:10:5)
    if let Some(open) = line.rfind('(') {
        if let Some(close) = line[open + 1..].find(')') {
            let inner = &line[open + 1..open + 1 + close];
            if let Some((url, line_1, col_1)) = split_url_line_col(inner) {
                return Some((&line[..=open], url, line_1, col_1, &line[open + 1 + close..]));
            }
        }
    }

    // SpiderMonkey / bare: foo@http://…:10:5  or  http://…:10:5
    let start = line.find("http://")
        .or_else(|| line.find("https://"))
        .or_else(|| line.find("file://"))?;
    let rest = &line[start..];
    let end = rest.find([' ', '\t', ')']).unwrap_or(rest.len());
    let loc = &rest[..end];
    let (url, line_1, col_1) = split_url_line_col(loc)?;
    Some((&line[..start], url, line_1, col_1, &rest[end..]))
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

fn load_map_for_url(url: &str) -> Option<Vec<u8>> {
    let map_url = format!("{url}.map");
    if let Some(path) = map_url.strip_prefix("file://") {
        return std::fs::read(path).ok();
    }

    #[cfg(all(feature = "fetch", not(target_arch = "wasm32")))]
    if map_url.starts_with("http://") || map_url.starts_with("https://") {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .ok()?;
        let resp = client.get(&map_url).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        return resp.bytes().ok().map(|b| b.to_vec());
    }

    let _ = map_url;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn enrich_noop_without_maps() {
        let stack = Some("Error: boom\n    at mystery".to_string());
        let out = enrich_stack_with_sourcemaps(stack.clone());
        assert_eq!(out, stack);
    }
}
