//! `style.rs`: tokens written as **basic Rust** — constants, which reads
//! like the Rust a theme author already knows and stays declarative (it's
//! parsed, never compiled or run):
//!
//! ```text
//! // Neubrutalism, light
//! pub const BACKGROUND: Color = rgb(255, 251, 235);
//! pub const ACCENT: Color = hex("#ffde59");
//! pub const RADIUS: f32 = 6.0;
//! pub const GLASS: bool = false;
//! pub const FONT_FAMILY: &str = "Archivo Black";
//! pub const CARD: Panel = Panel { fill: rgb(255, 255, 255), border_width: 3.0, shadow_x: 5.0, shadow_y: 5.0 };
//!
//! pub mod dark {
//!     pub const BACKGROUND: Color = rgb(24, 24, 27);
//! }
//! ```
//!
//! A constant's name in lower case is the token (`FONT_FAMILY` →
//! `font_family`); a `Panel { … }` value styles that panel (`CARD` →
//! `panels.card`). Values: `rgb(r, g, b)`, `Color::from_rgb8(r, g, b)`,
//! `hex("#…")` or a `"#…"` string, numbers, `true`/`false`, strings.

use serde_json::{json, Map, Value as Json};

/// Tokens from `style.rs` (dark-mode constants under `"dark"`).
pub fn parse(src: &str) -> Result<Json, String> {
    let clean = strip_comments(src);
    let mut out = Map::new();
    let mut dark = Map::new();
    let mut rest = clean.as_str();
    let mut in_dark = false;
    let line_of = |pos: usize| clean[..pos].matches('\n').count() + 1;
    loop {
        let t = rest.trim_start();
        if t.is_empty() {
            break;
        }
        let pos = clean.len() - t.len();
        if let Some(after) = t.strip_prefix("pub mod dark").or_else(|| t.strip_prefix("mod dark")) {
            let after = after.trim_start();
            let Some(a) = after.strip_prefix('{') else { return Err(format!("line {}: expected `{{` after `mod dark`", line_of(pos))) };
            if in_dark {
                return Err(format!("line {}: `mod dark` inside `mod dark`", line_of(pos)));
            }
            in_dark = true;
            rest = a;
            continue;
        }
        if let Some(a) = t.strip_prefix('}') {
            if !in_dark {
                return Err(format!("line {}: unexpected `}}`", line_of(pos)));
            }
            in_dark = false;
            rest = a;
            continue;
        }
        let decl = t.strip_prefix("pub ").unwrap_or(t);
        let Some(decl) = decl.strip_prefix("const ").or_else(|| decl.strip_prefix("static ")) else {
            return Err(format!("line {}: style.rs holds only `const NAME: Type = value;` (and `mod dark {{ … }}`)", line_of(pos)));
        };
        let end = find_semicolon(decl).ok_or(format!("line {}: missing `;`", line_of(pos)))?;
        let stmt = &decl[..end];
        rest = &decl[end + 1..];
        let (head, value) = stmt.split_once('=').ok_or(format!("line {}: a constant needs `= value`", line_of(pos)))?;
        let (name, ty) = head.split_once(':').map(|(n, t)| (n.trim(), t.trim())).unwrap_or((head.trim(), ""));
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("line {}: `{name}` isn't a constant name", line_of(pos)));
        }
        let key = name.to_lowercase();
        let v = if ty == "Panel" || value.trim_start().starts_with("Panel") {
            let inner = value.trim().strip_prefix("Panel").map(str::trim).and_then(|s| s.strip_prefix('{')).and_then(|s| s.strip_suffix('}')).ok_or(format!("line {}: a Panel is `Panel {{ field: value, … }}`", line_of(pos)))?;
            let mut m = Map::new();
            for field in split_top(inner, ',') {
                if field.trim().is_empty() {
                    continue;
                }
                let (k, v) = field.split_once(':').ok_or(format!("line {}: `{}` should be `field: value`", line_of(pos), field.trim()))?;
                m.insert(k.trim().to_string(), value_of(v.trim()).map_err(|e| format!("line {}: {e}", line_of(pos)))?);
            }
            let target = if in_dark { &mut dark } else { &mut out };
            let panels = target.entry("panels").or_insert_with(|| json!({})).as_object_mut().unwrap();
            panels.insert(key, Json::Object(m));
            continue;
        } else {
            value_of(value.trim()).map_err(|e| format!("line {}: {e}", line_of(pos)))?
        };
        if in_dark {
            dark.insert(key, v);
        } else {
            out.insert(key, v);
        }
    }
    if in_dark {
        return Err("`mod dark {` is never closed".into());
    }
    if !dark.is_empty() {
        out.insert("dark".into(), Json::Object(dark));
    }
    Ok(Json::Object(out))
}

fn value_of(v: &str) -> Result<Json, String> {
    let call = |prefix: &str| -> Option<Vec<String>> {
        let inner = v.strip_prefix(prefix)?.trim_start().strip_prefix('(')?.strip_suffix(')')?;
        Some(split_top(inner, ',').into_iter().map(|s| s.trim().to_string()).collect())
    };
    if let Some(args) = call("rgb").or_else(|| call("Color::from_rgb8")) {
        let n: Vec<u8> = args.iter().map(|a| a.trim_end_matches("u8").parse::<u8>()).collect::<Result<_, _>>().map_err(|_| format!("`{v}`: rgb takes three numbers 0–255"))?;
        if n.len() != 3 {
            return Err(format!("`{v}`: rgb takes three numbers"));
        }
        return Ok(json!(format!("#{:02x}{:02x}{:02x}", n[0], n[1], n[2])));
    }
    if let Some(args) = call("hex") {
        let s = args.first().map(|a| a.trim_matches('"').to_string()).unwrap_or_default();
        return color_string(&s);
    }
    if v == "true" || v == "false" {
        return Ok(json!(v == "true"));
    }
    if let Some(s) = v.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Ok(if s.starts_with('#') { color_string(s)? } else { json!(s) });
    }
    let num = v.trim_end_matches("f32").trim_end_matches("f64").trim_end_matches("_u32").trim_end_matches("u32").replace('_', "");
    if let Ok(n) = num.parse::<f64>() {
        return Ok(json!(n));
    }
    Err(format!("`{v}` isn't a value style.rs understands (rgb(), hex(), a number, true/false or a string)"))
}

fn color_string(s: &str) -> Result<Json, String> {
    let h = s.trim_start_matches('#');
    let full = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect(),
        6 => h.to_string(),
        _ => return Err(format!("`{s}` isn't a #rgb or #rrggbb colour")),
    };
    if !full.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("`{s}` isn't a colour"));
    }
    Ok(json!(format!("#{}", full.to_lowercase())))
}

fn strip_comments(src: &str) -> String {
    let mut out = String::new();
    let mut chars = src.chars().peekable();
    let mut in_str = false;
    while let Some(c) = chars.next() {
        if c == '"' {
            in_str = !in_str;
        }
        if !in_str && c == '/' && chars.peek() == Some(&'/') {
            for d in chars.by_ref() {
                if d == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if !in_str && c == '#' && chars.peek() == Some(&'[') {
            // Attributes (`#[allow(dead_code)]`) are ignored.
            for d in chars.by_ref() {
                if d == ']' {
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn find_semicolon(s: &str) -> Option<usize> {
    let mut in_str = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_str = !in_str,
            ';' if !in_str => return Some(i),
            _ => {}
        }
    }
    None
}

fn split_top(s: &str, sep: char) -> Vec<&str> {
    let (mut depth, mut start, mut in_str) = (0i32, 0, false);
    let mut out = vec![];
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '(' | '{' | '[' if !in_str => depth += 1,
            ')' | '}' | ']' if !in_str => depth -= 1,
            c if c == sep && depth == 0 && !in_str => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_constants_panels_and_dark() {
        let v = parse(
            r##"
// comment
#[allow(dead_code)]
pub const BACKGROUND: Color = rgb(255, 251, 235);
pub const ACCENT: Color = hex("#FFDE59");
const BORDER: Color = "#000";
pub const RADIUS: f32 = 6.0;
pub const GLASS: bool = false;
pub const FONT_FAMILY: &str = "Archivo Black";
pub const CARD: Panel = Panel { fill: rgb(255, 255, 255), border_width: 3.0, shadow_x: 5.0 };
pub mod dark {
    pub const BACKGROUND: Color = Color::from_rgb8(24, 24, 27);
}
"##,
        )
        .unwrap();
        assert_eq!(v["background"], "#fffbeb");
        assert_eq!(v["accent"], "#ffde59");
        assert_eq!(v["border"], "#000000");
        assert_eq!(v["radius"], 6.0);
        assert_eq!(v["glass"], false);
        assert_eq!(v["font_family"], "Archivo Black");
        assert_eq!(v["panels"]["card"]["border_width"], 3.0);
        assert_eq!(v["dark"]["background"], "#18181b");
        assert!(parse("fn main() {}").unwrap_err().contains("holds only"));
        assert!(parse("const X: Color = rgb(1, 2);").unwrap_err().contains("three numbers"));
    }
}
