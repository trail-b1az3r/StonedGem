//! Theme Studio end to end, without a window: it installs as a built-in
//! power extension, its page's messages reach its Zsharpend script, which
//! compiles the graph through the power API, saves a .hypertheme into the
//! profile and asks the browser to use it; the palette then follows it.

use serde_json::json;
use stonedgem_chrome::hyper;
use stonedgem_chrome::settings::Settings;
use stonedgem_chrome::zs_hooks::{Action, Scripts};

#[test]
fn studio_compiles_saves_and_applies() {
    let root = std::env::temp_dir().join(format!("stonedgem-studio-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    std::env::set_var("XDG_CACHE_HOME", root.join("cache"));

    let mut list = vec![];
    let id = hyper::install_studio(&mut list).expect("install Theme Studio");
    let e = list.iter().find(|e| e.id == id).unwrap().clone();
    assert!(e.power && e.builtin && e.has_script, "{e:?}");
    let page = std::fs::read_to_string(std::path::Path::new(&e.path).join("extra.html")).unwrap();
    assert!(page.contains("\"Neubrutalism\"") && !page.contains("/*NEUBRUTALISM_GRAPH*/"), "the template graph is filled in");
    // Installing again with the same version keeps it.
    assert_eq!(hyper::install_studio(&mut list).unwrap(), id);
    assert!(hyper::page_bridge(&e).contains("sendMessage"));

    let mut scripts = Scripts::load(&list);
    assert_eq!(scripts.len(), 1, "{:?}", scripts.errors);
    let graph: serde_json::Value = serde_json::from_str(include_str!("../../stonedgem-hypertheme/themes/neubrutalism/graph.json")).unwrap();
    let url = format!("file://{}/extra.html", e.path);

    // compile: every part, the preview CSS and each node's value.
    let (r, actions) = scripts.message(&id, json!({ "kind": "compile", "graph": graph, "dark": false }), &url).unwrap();
    let r = r.expect("compile");
    assert!(actions.is_empty());
    assert_eq!(r["name"], "Neubrutalism");
    assert!(r["files"]["style.rs"].as_str().unwrap().contains("pub const BORDER_WIDTH: f32 = 3.0;"));
    let css = r["css"].as_str().unwrap();
    assert!(css.contains("--ht-background: #fff4dc") && css.contains(".ht-toolbar { background: #ffde59"), "{css}");
    assert_eq!(r["values"]["on_pink"], "#111111");
    assert_eq!(r["values"]["alt"], "#ffeaa1");
    assert_eq!(r["warnings"], json!([]));

    // A shader node's preview.
    let (r, _) = scripts.message(&id, json!({ "kind": "shader", "code": "fragment preview(uv: vec2) -> color { return vec4(uv.x, 0.2, 0.4, 1.0) }" }), &url).unwrap();
    assert!(r.unwrap()["image"].as_str().unwrap().starts_with("data:image/png;base64,"));

    // apply: saved into the profile, then the browser is asked to use it.
    let mut g = graph.clone();
    for n in g["nodes"].as_array_mut().unwrap() {
        if n["id"] == "light" {
            n["props"]["name"] = json!("My Brutal");
        }
        if n["id"] == "pink" {
            n["props"]["value"] = json!("#7c5cff");
        }
    }
    let (r, actions) = scripts.message(&id, json!({ "kind": "apply", "graph": g }), &url).unwrap();
    let file = r.expect("apply")["file"].as_str().unwrap().to_string();
    assert_eq!(file, "my-brutal.hypertheme");
    assert!(actions.contains(&Action::ApplyHypertheme(file.clone())), "{actions:?}");
    assert!(stonedgem_hypertheme::available().iter().any(|(f, n)| f == &file && n == "My Brutal"));
    let saved = stonedgem_hypertheme::load(&file).unwrap();
    assert!(saved.files.contains_key("graph.json"), "saved with its graph, to edit again");

    // The window's palette follows the theme.
    let s = Settings { hypertheme: file, dark: false, ..Default::default() };
    let (p, in_use, err) = hyper::palette(&s);
    assert!(err.is_none(), "{err:?}");
    assert!(in_use.is_some() && !p.glass && p.edge == 3.0 && p.hard == Some((4.0, 4.0)));
    let accent = p.primary;
    assert!((accent.r - 0x7c as f32 / 255.0).abs() < 0.01, "the new accent: {accent:?}");
    let url = hyper::start_page(in_use.as_ref().unwrap(), "StonedGem").unwrap();
    let html = std::fs::read_to_string(url.trim_start_matches("file://")).unwrap();
    assert!(html.contains("data:image/png;base64,") && html.contains("ht-start"), "the backdrop shader and the TSX start page");

    // The built-in theme, and a missing one falls back with the reason.
    let s = Settings { hypertheme: "neubrutalism.hypertheme".into(), dark: true, ..Default::default() };
    let (p, _, err) = hyper::palette(&s);
    assert!(err.is_none() && p.dark && p.toolbar.is_some());
    let (_, none, err) = hyper::palette(&Settings { hypertheme: "gone.hypertheme".into(), ..Default::default() });
    assert!(none.is_none() && err.unwrap().contains("no hypertheme"));
    let _ = std::fs::remove_dir_all(&root);
}
