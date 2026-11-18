//! WebExtension match patterns (`*://*.example.com/path*`, `<all_urls>`),
//! for deciding which pages an extension's content script runs on — in
//! Rust ([`matches`]) and as a JavaScript guard the engine can run in the
//! page ([`guard_js`]), so a script stays correct after the page navigates.
//!
//! Rules (as in Chrome and Firefox): the scheme is `*` (http or https) or
//! a given scheme; the host is `*`, `*.domain` (the domain and every
//! subdomain) or exact; the path is a glob where `*` is any run of
//! characters. Ports are ignored.

/// Split a URL into (scheme, host, path-and-query).
fn parts(url: &str) -> Option<(&str, &str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    let path = path.split('#').next().unwrap_or(path);
    Some((scheme, host, path))
}

/// `*` = any run of characters.
fn glob(p: &str, t: &str) -> bool {
    let (p, t) = (p.as_bytes(), t.as_bytes());
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

/// Does `url` match the pattern?
pub fn matches(pattern: &str, url: &str) -> bool {
    let Some((us, uh, up)) = parts(url) else { return false };
    if pattern == "<all_urls>" {
        return matches!(us, "http" | "https" | "file" | "ftp");
    }
    let Some((ps, ph, pp)) = parts(pattern) else { return false };
    let scheme_ok = if ps == "*" { us == "http" || us == "https" } else { ps.eq_ignore_ascii_case(us) };
    let uh = uh.to_ascii_lowercase();
    let ph = ph.to_ascii_lowercase();
    let host_ok = if ph == "*" {
        true
    } else if let Some(base) = ph.strip_prefix("*.") {
        uh == base || uh.ends_with(&format!(".{base}"))
    } else {
        uh == ph
    };
    scheme_ok && host_ok && glob(pp, up)
}

/// Runs on `url` if a `matches` pattern matches and no `excludes` one does.
pub fn applies(matches_: &[String], excludes: &[String], url: &str) -> bool {
    matches_.iter().any(|m| matches(m, url)) && !excludes.iter().any(|x| matches(x, url))
}

fn escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if "\\^$.|?+()[]{}/".contains(c) {
            o.push('\\');
        }
        o.push(c);
    }
    o
}

/// The pattern as a JavaScript regular expression source (for
/// `new RegExp(...)`), matching what [`matches`] does.
pub fn to_regex(pattern: &str) -> Option<String> {
    if pattern == "<all_urls>" {
        return Some("^(https?|file|ftp)://".into());
    }
    let (ps, ph, pp) = parts(pattern)?;
    let scheme = if ps == "*" { "https?".to_string() } else { escape(&ps.to_ascii_lowercase()) };
    let host = if ph == "*" {
        "[^/]*".to_string()
    } else if let Some(base) = ph.strip_prefix("*.") {
        format!("([^/@]*\\.)?{}", escape(&base.to_ascii_lowercase()))
    } else {
        escape(&ph.to_ascii_lowercase())
    };
    let path: String = pp.split('*').map(escape).collect::<Vec<_>>().join(".*");
    Some(format!("^{scheme}://([^/@]*@)?{host}(:\\d+)?{path}(#.*)?$"))
}

/// A JavaScript expression that is true when the page's address matches:
/// `(… ).test(location.href)`, with excludes.
pub fn guard_js(matches_: &[String], excludes: &[String]) -> String {
    let list = |v: &[String]| -> String {
        let r: Vec<String> = v.iter().filter_map(|p| to_regex(p)).map(|r| serde_json::to_string(&r).unwrap_or_default()).collect();
        format!("[{}]", r.join(","))
    };
    format!(
        "(function(h){{var t=function(a){{return a.some(function(r){{return new RegExp(r,'i').test(h);}});}};return t({})&&!t({});}})(location.href)",
        list(matches_),
        list(excludes)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns() {
        assert!(matches("*://*/*", "https://example.com/a"));
        assert!(matches("*://*/*", "http://x.org/"));
        assert!(!matches("*://*/*", "file:///etc/passwd"));
        assert!(matches("<all_urls>", "file:///home/a.html"));
        assert!(matches("https://*.example.com/*", "https://example.com/"));
        assert!(matches("https://*.example.com/*", "https://a.b.example.com/x?y"));
        assert!(!matches("https://*.example.com/*", "https://notexample.com/"));
        assert!(!matches("https://*.example.com/*", "http://example.com/"));
        assert!(matches("*://docs.example.org/guide/*", "https://docs.example.org/guide/intro#top"));
        assert!(!matches("*://docs.example.org/guide/*", "https://docs.example.org/blog/"));
        assert!(matches("*://example.com/*", "https://example.com:8443/x"));
        assert!(matches("*://example.com/*.pdf", "https://example.com/a/b.pdf"));
        assert!(applies(&["*://*/*".into()], &["*://*.bank.com/*".into()], "https://news.org/"));
        assert!(!applies(&["*://*/*".into()], &["*://*.bank.com/*".into()], "https://my.bank.com/"));
    }

    #[test]
    fn regex_shape() {
        assert_eq!(to_regex("*://*.example.com/*").unwrap(), r"^https?://([^/@]*@)?([^/@]*\.)?example\.com(:\d+)?\/.*(#.*)?$");
        assert!(guard_js(&["*://*/*".into()], &[]).contains(".test(h)"));
    }
}
