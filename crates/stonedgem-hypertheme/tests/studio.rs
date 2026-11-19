//! Theme Studio's script turns a node graph into a hypertheme. The built-in
//! Neubrutalism theme is its output for `themes/neubrutalism/graph.json`:
//! this test runs the script again and checks every part matches.
//! `REGENERATE=1 cargo test -p stonedgem-hypertheme --test studio` rewrites them.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value as Json;
use stonedgem_hypertheme::{json_to_zxx, zxx_to_json, Hypertheme};

fn generate(graph: &Json) -> Result<(String, BTreeMap<String, String>), String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("studio");
    let read = move |p: &str| std::fs::read_to_string(dir.join(p)).ok();
    let mut prog = zsharpend::Program::load("script.zxx", &read, |it| {
        for (name, _) in stonedgem_extensions::zscript::API.iter().chain(stonedgem_extensions::zscript::POWER_API) {
            it.register(name, |_, _| Ok(zsharpend::Value::Null));
        }
    })?;
    let out = prog.interp.call_global("generate", vec![json_to_zxx(graph)])?.ok_or("no generate()")?;
    let out = zxx_to_json(&out);
    let files = out["files"].as_object().ok_or("files isn't a map")?.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect();
    Ok((out["name"].as_str().unwrap_or("").to_string(), files))
}

#[test]
fn neubrutalism_comes_from_its_graph() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("themes/neubrutalism");
    let graph_text = std::fs::read_to_string(dir.join("graph.json")).unwrap();
    let graph: Json = serde_json::from_str(&graph_text).unwrap();
    let (name, mut files) = generate(&graph).unwrap();
    assert_eq!(name, "Neubrutalism");
    files.insert("graph.json".into(), graph_text);
    if std::env::var_os("REGENERATE").is_some() {
        for (f, text) in &files {
            std::fs::write(dir.join(f), text).unwrap();
        }
    }
    for (f, text) in &files {
        let on_disk = std::fs::read_to_string(dir.join(f)).unwrap_or_default();
        assert_eq!(&on_disk, text, "{f} differs from what Theme Studio makes of graph.json (REGENERATE=1 to rewrite)");
    }
    let h = Hypertheme::from_files(files).expect("a valid hypertheme");
    let light = h.resolve(false);
    assert!(light.warnings.is_empty(), "{:?}", light.warnings);
    assert_eq!(light.color("background").unwrap(), "#fff4dc");
    assert_eq!(light.color("on_accent").unwrap(), "#111111", "readable node");
    assert_eq!(light.number("border_width"), Some(3.0));
    assert_eq!(light.flag("glass"), Some(false));
    assert_eq!(light.panel("toolbar", "fill").unwrap(), "#ffde59");
    assert_eq!(light.panel("card", "shadow_x").unwrap(), 6.0);
    let dark = h.resolve(true);
    assert!(dark.warnings.is_empty(), "{:?}", dark.warnings);
    assert_eq!(dark.color("background").unwrap(), "#18181b");
    assert_eq!(dark.panel("toolbar", "fill").unwrap(), "#27272a");
    assert_eq!(dark.panel("button", "fill").unwrap(), "#5ce1e6", "kept from light");
    assert_ne!(dark.color("danger"), light.color("danger"), "theme.zxx lifts it");
    assert!(stonedgem_hypertheme::background_png(&h, 64, 36).unwrap().is_ok());
}

#[test]
fn bad_graphs_say_why() {
    let e = generate(&serde_json::json!({ "nodes": [], "wires": [] })).unwrap_err();
    assert!(e.contains("Theme node"), "{e}");
    let looped = serde_json::json!({
        "nodes": [
            { "id": "a", "type": "shade", "props": {} },
            { "id": "b", "type": "shade", "props": {} },
            { "id": "t", "type": "theme", "props": { "name": "Loop" } }
        ],
        "wires": [ { "from": "a", "to": "b", "in": "color" }, { "from": "b", "to": "a", "in": "color" }, { "from": "a", "to": "t", "in": "accent" } ]
    });
    let e = generate(&looped).unwrap_err();
    assert!(e.contains("loop"), "{e}");
}
