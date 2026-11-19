//! Builds the Z#++ documentation site.
//!
//! Pages are Markdown in `docs/zsharp/`, ordered by `docs/zsharp/nav.txt`.
//! Fenced code blocks are checked against the real implementation:
//!
//! | Info string            | What happens                                              |
//! |------------------------|-----------------------------------------------------------|
//! | `zsp`                  | parsed as a program                                       |
//! | `zsp run`              | loaded and `Main()` run; its output (and canvas) shown    |
//! | `zsp error`            | must fail; the error message is shown                     |
//! | `zsx`                  | parsed as a header                                        |
//! | `zsg`                  | parsed, type-checked and compiled to WGSL (naga-validated)|
//! | `… include=path`       | the code is that file from the repository                 |
//! | `rust`, `text`, `sh`…  | shown as is                                               |
//!
//! A failing example fails the build, so the docs can't drift from the
//! language.

pub mod highlight;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pulldown_cmark::{CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use stonedgem_zsharp::ast::Kind;
use stonedgem_zsharp::{Program, Value};

use highlight::escape;

const STYLE: &str = include_str!("../assets/style.css");
const SCRIPT: &str = include_str!("../assets/site.js");
const SOURCE_BASE: &str = "https://github.com/trail-b1az3r/StonedGem/blob/main/";

pub struct Page {
    pub file: String,
    pub section: String,
    pub title: String,
    pub md: String,
}

impl Page {
    fn out_name(&self) -> String {
        self.file.trim_end_matches(".md").to_string() + ".html"
    }
}

/// What a build produced.
#[derive(Default, Debug)]
pub struct Report {
    pub pages: usize,
    pub examples_run: usize,
    pub examples_checked: usize,
}

/// Read `docs/zsharp/nav.txt`: `# Section` lines, then page files.
pub fn load_pages(root: &Path) -> Result<Vec<Page>, String> {
    let dir = root.join("docs/zsharp");
    let nav = std::fs::read_to_string(dir.join("nav.txt")).map_err(|e| format!("docs/zsharp/nav.txt: {e}"))?;
    let mut section = String::new();
    let mut pages = vec![];
    for line in nav.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(s) = line.strip_prefix("# ") {
            section = s.to_string();
            continue;
        }
        let md = std::fs::read_to_string(dir.join(line)).map_err(|e| format!("docs/zsharp/{line}: {e}"))?;
        let title = md.lines().find_map(|l| l.strip_prefix("# ")).ok_or(format!("docs/zsharp/{line}: no `# Title` line"))?.trim().to_string();
        pages.push(Page { file: line.to_string(), section: section.clone(), title, md });
    }
    Ok(pages)
}

struct Example<'a> {
    lang: &'a str,
    run: bool,
    error: bool,
    include: Option<String>,
}

fn parse_info(info: &str) -> Example<'_> {
    let mut words = info.split_whitespace();
    let lang = words.next().unwrap_or("");
    let mut ex = Example { lang, run: false, error: false, include: None };
    for w in words {
        match w {
            "run" => ex.run = true,
            "error" => ex.error = true,
            w if w.starts_with("include=") => ex.include = Some(w["include=".len()..].to_string()),
            _ => {}
        }
    }
    ex
}

/// What running an example produced.
struct Ran {
    output: String,
    png: Option<Vec<u8>>,
    error: Option<String>,
}

fn canvas_png(p: &Program) -> Option<Vec<u8>> {
    ["screen", "canvas"].iter().find_map(|g| match p.interp.global(g) {
        Some(Value::Object(o)) => o.as_any().downcast_ref::<stonedgem_zsharp::canvas::Canvas>().and_then(|c| c.png().ok()),
        _ => None,
    })
}

fn check(root: &Path, ex: &Example, code: &str, report: &mut Report) -> Result<Option<Ran>, String> {
    let (name, dir) = match &ex.include {
        Some(p) => {
            let full = root.join(p);
            let name = full.file_name().and_then(|n| n.to_str()).unwrap_or("main.zsp").to_string();
            (name, full.parent().map(Path::to_path_buf).unwrap_or_default())
        }
        None => (format!("main.{}", ex.lang), PathBuf::new()),
    };
    let read = {
        let (dir, code, name) = (dir.clone(), code.to_string(), name.clone());
        move |p: &str| if p == name { Some(code.clone()) } else { std::fs::read_to_string(dir.join(p)).ok() }
    };
    match ex.lang {
        "zsp" | "zs" if ex.run || ex.error => {
            report.examples_run += 1;
            let r = Program::load(&name, &read, |_| {}).and_then(|mut p| {
                let r = p.run_main();
                let output = p.output();
                let png = canvas_png(&p);
                r.map(|_| (output, png)).map_err(|e| format!("{}{e}", p.output()))
            });
            match (r, ex.error) {
                (Ok((output, png)), false) => Ok(Some(Ran { output, png, error: None })),
                (Err(e), true) => Ok(Some(Ran { output: String::new(), png: None, error: Some(e) })),
                (Ok(_), true) => Err("this example is marked `error` but runs without one".into()),
                (Err(e), false) => Err(e),
            }
        }
        "zsp" | "zs" => {
            report.examples_checked += 1;
            match &ex.include {
                Some(_) => Program::load(&name, &read, |_| {}).map(|_| None),
                None => stonedgem_zsharp::parser::parse(code, Kind::Program).map(|_| None),
            }
        }
        "zsx" | "zsg" => {
            report.examples_checked += 1;
            let r = if ex.lang == "zsx" {
                stonedgem_zsharp::parser::parse(code, Kind::Header).map(|_| ())
            } else {
                stonedgem_zsharp::zsg::Module::parse(code).and_then(|m| m.validate()).map(|_| ())
            };
            match (r, ex.error) {
                (Ok(()), false) => Ok(None),
                (Err(e), true) => Ok(Some(Ran { output: String::new(), png: None, error: Some(e) })),
                (Ok(()), true) => Err("this example is marked `error` but has none".into()),
                (Err(e), false) => Err(e),
            }
        }
        _ => Ok(None),
    }
}

fn slug(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            o.extend(c.to_lowercase());
        } else if (c == ' ' || c == '-' || c == '_') && !o.ends_with('-') {
            o.push('-');
        }
    }
    o.trim_matches('-').to_string()
}

struct Rendered {
    html: String,
    toc: Vec<(String, String)>,
    images: Vec<(String, Vec<u8>)>,
}

fn render(root: &Path, page: &Page, report: &mut Report) -> Result<Rendered, String> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut events: Vec<Event> = vec![];
    let mut toc = vec![];
    let mut images = vec![];
    let mut code: Option<(String, String)> = None;
    let mut heading: Option<(HeadingLevel, Vec<Event>)> = None;
    let mut used: HashMap<String, usize> = HashMap::new();
    let stem = page.file.trim_end_matches(".md");
    let mut figure = 0;

    for ev in Parser::new_ext(&page.md, opts) {
        if let Some((_, buf)) = &mut code {
            match ev {
                Event::Text(t) => buf.push_str(&t),
                Event::End(TagEnd::CodeBlock) => {
                    let (info, text) = code.take().unwrap();
                    let ex = parse_info(&info);
                    let text = match &ex.include {
                        Some(p) => std::fs::read_to_string(root.join(p)).map_err(|e| format!("{}: include={p}: {e}", page.file))?,
                        None => text,
                    };
                    let ran = check(root, &ex, &text, report).map_err(|e| {
                        let first = text.lines().next().unwrap_or("");
                        format!("{}: example starting `{first}`: {e}", page.file)
                    })?;
                    let body = match ex.lang {
                        "zsp" | "zs" | "zsx" | "zsg" => highlight::zsharp(&text),
                        "rust" | "rs" => highlight::rust(&text),
                        _ => escape(&text),
                    };
                    let label = match (&ex.include, ex.lang) {
                        (Some(p), _) => format!("<a href=\"{SOURCE_BASE}{p}\">{}</a>", escape(p)),
                        (None, "zsp" | "zs") => "program".into(),
                        (None, "zsx") => "header · .zsx".into(),
                        (None, "zsg") => "GPU code · .zsg".into(),
                        (None, "sh") => "terminal".into(),
                        (None, "rust") => "Rust".into(),
                        (None, "text" | "") => String::new(),
                        (None, l) => escape(l),
                    };
                    let mut html = format!(
                        "<figure class=\"code\"><figcaption><span>{label}</span><button class=\"copy\" type=\"button\" aria-label=\"Copy code\">Copy</button></figcaption><pre><code class=\"lang-{}\">{}</code></pre>",
                        escape(ex.lang),
                        body.trim_end()
                    );
                    if let Some(r) = ran {
                        if let Some(e) = r.error {
                            html.push_str(&format!("<div class=\"out err\"><span class=\"out-label\">Error</span><pre>{}</pre></div>", escape(e.trim_end())));
                        } else {
                            if !r.output.is_empty() {
                                html.push_str(&format!("<div class=\"out\"><span class=\"out-label\">Output</span><pre>{}</pre></div>", escape(r.output.trim_end())));
                            }
                            if let Some(png) = r.png {
                                figure += 1;
                                let f = format!("img/{stem}-{figure}.png");
                                html.push_str(&format!("<div class=\"out canvas\"><span class=\"out-label\">Canvas</span><img src=\"{f}\" alt=\"What this program draws\" loading=\"lazy\"></div>"));
                                images.push((f, png));
                            }
                        }
                    }
                    html.push_str("</figure>");
                    events.push(Event::Html(CowStr::from(html)));
                }
                _ => {}
            }
            continue;
        }
        if let Some((level, buf)) = &mut heading {
            if let Event::End(TagEnd::Heading(_)) = ev {
                let level = *level;
                let text: String = buf.iter().filter_map(|e| match e {
                    Event::Text(t) | Event::Code(t) => Some(t.to_string()),
                    _ => None,
                }).collect();
                let mut inner = String::new();
                pulldown_cmark::html::push_html(&mut inner, buf.drain(..));
                heading = None;
                if level == HeadingLevel::H1 {
                    events.push(Event::Html(CowStr::from(format!("<h1>{inner}</h1>"))));
                    continue;
                }
                let base = slug(&text);
                let n = used.entry(base.clone()).or_insert(0);
                let id = if *n == 0 { base.clone() } else { format!("{base}-{n}") };
                *n += 1;
                if level == HeadingLevel::H2 {
                    toc.push((id.clone(), text.clone()));
                }
                let lv = level as usize;
                events.push(Event::Html(CowStr::from(format!(
                    "<h{lv} id=\"{id}\"><a class=\"anchor\" href=\"#{id}\" aria-hidden=\"true\">#</a>{inner}</h{lv}>"
                ))));
            } else {
                buf.push(ev);
            }
            continue;
        }
        match ev {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => code = Some((info.to_string(), String::new())),
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => code = Some(("text".into(), String::new())),
            Event::Start(Tag::Heading { level, .. }) => heading = Some((level, vec![])),
            Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
                // Links between pages: page.md → page.html
                let d = dest_url.to_string();
                let dest = if !d.contains("://") && d.contains(".md") { d.replacen(".md", ".html", 1) } else { d };
                events.push(Event::Start(Tag::Link { link_type, dest_url: CowStr::from(dest), title, id }));
            }
            Event::Code(t) => events.push(Event::Html(CowStr::from(format!("<code>{}</code>", escape(&t))))),
            ev => events.push(ev),
        }
    }
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    // Callouts: a blockquote whose first word is bold "Note"/"Careful".
    let html = html
        .replace("<blockquote>\n<p><strong>Note</strong>", "<blockquote class=\"note\">\n<p><strong>Note</strong>")
        .replace("<blockquote>\n<p><strong>Careful</strong>", "<blockquote class=\"careful\">\n<p><strong>Careful</strong>");
    Ok(Rendered { html, toc, images })
}

fn nav_html(pages: &[Page], current: usize) -> String {
    let mut o = String::new();
    let mut section = "";
    for (i, p) in pages.iter().enumerate() {
        if p.section != section {
            if !section.is_empty() {
                o.push_str("</ul>");
            }
            section = &p.section;
            o.push_str(&format!("<p class=\"nav-section\">{}</p><ul>", escape(section)));
        }
        let cur = if i == current { " aria-current=\"page\"" } else { "" };
        o.push_str(&format!("<li><a href=\"{}\"{cur}>{}</a></li>", p.out_name(), escape(&p.title)));
    }
    o.push_str("</ul>");
    o
}

fn page_html(pages: &[Page], i: usize, r: &Rendered) -> String {
    let p = &pages[i];
    let toc = if r.toc.len() > 1 {
        let items: String = r.toc.iter().map(|(id, t)| format!("<li><a href=\"#{id}\">{}</a></li>", escape(t))).collect();
        format!("<nav class=\"toc\" aria-label=\"On this page\"><p>On this page</p><ul>{items}</ul></nav>")
    } else {
        String::new()
    };
    let prev = i.checked_sub(1).map(|j| format!("<a class=\"prev\" href=\"{}\"><span>Previous</span>{}</a>", pages[j].out_name(), escape(&pages[j].title))).unwrap_or_default();
    let next = pages.get(i + 1).map(|n| format!("<a class=\"next\" href=\"{}\"><span>Next</span>{}</a>", n.out_name(), escape(&n.title))).unwrap_or_default();
    let title = if i == 0 { "Z#++ documentation".to_string() } else { format!("{} · Z#++", p.title) };
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="Z#++ — StonedGem's sandboxed scripting language: programs, headers, 2D drawing and GPU code.">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:ital,wght@0,400;0,500;1,400&family=IBM+Plex+Sans:wght@400;500;600&display=swap">
<link rel="stylesheet" href="style.css">
<script>try{{var t=localStorage.getItem('zs-theme');if(t)document.documentElement.dataset.theme=t}}catch(e){{}}</script>
</head>
<body>
<a class="skip" href="#main">Skip to content</a>
<header class="top">
  <a class="brand" href="index.html"><span class="mark">Z#++</span><span class="sub">docs</span></a>
  <span class="ver">StonedGem 0.1</span>
  <nav class="top-links" aria-label="Elsewhere">
    <a href="https://github.com/trail-b1az3r/StonedGem/tree/main/crates/stonedgem-zsharp">Source</a>
    <button class="theme" type="button" aria-label="Switch light or dark">Theme</button>
  </nav>
</header>
<div class="layout">
  <aside class="side">
    <details class="nav-wrap" open>
      <summary>Contents</summary>
      <nav aria-label="Documentation">{nav}</nav>
    </details>
  </aside>
  <main id="main" class="doc">
    <article>
{body}
    </article>
    <footer class="pager">{prev}{next}</footer>
    <p class="edit"><a href="{SOURCE_BASE}docs/zsharp/{file}">Edit this page</a> · Every example on this page was run by the Z#++ interpreter when the site was built.</p>
  </main>
  {toc}
</div>
<script src="site.js"></script>
</body>
</html>
"##,
        nav = nav_html(pages, i),
        body = r.html,
        file = p.file,
    )
}

/// Build the site from `root` (the repository) into `out`.
pub fn build(root: &Path, out: &Path) -> Result<Report, String> {
    let pages = load_pages(root)?;
    let mut report = Report { pages: pages.len(), ..Default::default() };
    std::fs::create_dir_all(out.join("img")).map_err(|e| e.to_string())?;
    let mut rendered = vec![];
    for p in &pages {
        rendered.push(render(root, p, &mut report)?);
    }
    for (i, r) in rendered.iter().enumerate() {
        std::fs::write(out.join(pages[i].out_name()), page_html(&pages, i, r)).map_err(|e| e.to_string())?;
        for (f, png) in &r.images {
            std::fs::write(out.join(f), png).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(out.join("style.css"), STYLE).map_err(|e| e.to_string())?;
    std::fs::write(out.join("site.js"), SCRIPT).map_err(|e| e.to_string())?;
    std::fs::write(out.join(".nojekyll"), "").map_err(|e| e.to_string())?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The whole site builds: every example parses, checks or runs.
    #[test]
    fn site_builds_and_examples_run() {
        let out = std::env::temp_dir().join(format!("zsharp-docs-{}", std::process::id()));
        let r = build(&repo(), &out).unwrap_or_else(|e| panic!("{e}"));
        assert!(r.pages >= 8 && r.examples_run >= 10, "{r:?}");
        assert!(out.join("index.html").exists() && out.join("style.css").exists());
    }

    /// Every built-in global is in the reference.
    #[test]
    fn reference_lists_every_builtin() {
        let reference = std::fs::read_to_string(repo().join("docs/zsharp/reference.md")).unwrap();
        let missing: Vec<&str> = stonedgem_zsharp::stdlib::GLOBALS.iter().copied().filter(|g| !reference.contains(&format!("`{g}")) ).collect();
        assert!(missing.is_empty(), "not in docs/zsharp/reference.md: {missing:?}");
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Strings & colours"), "strings-colours");
        assert_eq!(slug("`match`"), "match");
    }
}
