//! Power hyperthemes (.phypertheme): QML, global programs and shaders,
//! format, styles, pictures, and modules/headers the programs import.

use std::collections::BTreeMap;

use stonedgem_hypertheme::{Hypertheme, NEEDS_POWER_CONFIRM};

fn png(w: u32, h: u32) -> Vec<u8> {
    // A tiny valid PNG from the Z#++ canvas.
    let c = stonedgem_zsharp::canvas::Canvas::new(w as i64, h as i64).unwrap();
    c.png().unwrap()
}

fn parts(power: bool) -> (BTreeMap<String, String>, BTreeMap<String, Vec<u8>>) {
    let mut f = BTreeMap::new();
    let mut add = |k: &str, v: &str| {
        f.insert(k.to_string(), v.to_string());
    };
    add("hypertheme.toml", &format!("[hypertheme]\nname = \"Aurora Glass\"\npower = {power}\n"));
    add("tokens.json", r##"{ "background": "#0b1020", "text": "#e8ecff", "accent": "#7aa2ff", "on_accent": "#0b1020" }"##);
    add("modules/palette.zxx", "fn ice() { return #9fd4ff }\nfn blur() { return 22 }\n");
    add("include/shared.zsx", "const GLOW = 0.6\n");
    add("theme.zxx", "import palette\nfn tokens(dark) { return { surface_alt: palette.ice() } }\n");
    add("global.zxx", "import palette as palette\nfn global(dark) {\n    return { glass: { blur: palette.blur(), tint: \"#7aa2ff\" }, background_image: \"images/sky.png\", tokens: { danger: #ff6b8a } }\n}\n");
    add("global.zsg", "fragment global(uv: vec2) -> color { return vec4(0.5, 0.7, 1.0, uv.y * 0.2) }\n");
    add("global.qml", "import QtQuick\nItem { Rectangle { anchors.fill: parent; color: \"transparent\" } }\n");
    add("effect.qml", "import QtQuick\nimport QtQuick.Effects\nMultiEffect { property var source; saturation: -0.2 }\n");
    add("format-panel.qt", "import QtQuick\nQtObject { property int toolbarHeight: 52; property int tabHeight: 34 }\n");
    add("styles.tsx", "export const button = { radius: 6, weight: 700 };\nexport default { card: { padding: 20 } };\n");
    let mut a = BTreeMap::new();
    a.insert("images/sky.png".to_string(), png(8, 4));
    (f, a)
}

#[test]
fn power_parts_need_a_power_theme() {
    let (f, a) = parts(false);
    let e = Hypertheme::from_parts(f, a).unwrap_err();
    assert!(e.contains("power = true") && e.contains("global.qml"), "{e}");
    let (mut f, a) = parts(true);
    f.insert("images/evil.exe".into(), "x".into());
    assert!(Hypertheme::from_parts(f, a).unwrap_err().contains("images/evil.exe"));
    let (f, mut a) = parts(true);
    a.insert("images/fake.png".into(), b"not a picture".to_vec());
    assert!(Hypertheme::from_parts(f, a).unwrap_err().contains("isn't a PNG"));
    let (mut f, a) = parts(true);
    f.insert("../escape.zxx".into(), "".into());
    assert!(Hypertheme::from_parts(f, a).is_err());
}

#[test]
fn power_theme_resolves_installs_and_unpacks() {
    let root = std::env::temp_dir().join(format!("ht-power-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    let (f, a) = parts(true);
    let h = Hypertheme::from_parts(f, a).unwrap();
    assert!(h.is_power());
    assert_eq!(h.file_name(), "aurora-glass.phypertheme");

    // Programs import the theme's modules; global.zxx gives global settings and tokens.
    let r = h.resolve(true);
    assert_eq!(r.color("surface_alt").unwrap(), "#9fd4ff", "theme.zxx imported modules/palette.zxx: {:?}", r.warnings);
    assert_eq!(r.color("danger").unwrap(), "#ff6b8a", "tokens from global.zxx");
    assert_eq!(r.global["glass"]["blur"], 22);
    assert_eq!(r.global["background_image"], "images/sky.png");

    // The global shader, styles and the unpacked folder.
    let overlay = stonedgem_hypertheme::global_png(&h, 32, 16).unwrap().unwrap();
    assert!(overlay.starts_with(b"\x89PNG"));
    let js = h.styles_js().unwrap().unwrap();
    assert!(js.contains("exports.button = button") && js.contains("exports.default =") && js.trim_end().ends_with("return exports;"), "{js}");
    let dir = root.join("unpacked");
    h.materialize(&dir).unwrap();
    for f in ["effect.qml", "global.qml", "format-panel.qt", "modules/palette.zxx", "images/sky.png"] {
        assert!(dir.join(f).is_file(), "{f}");
    }

    // Zip round trip keeps the pictures; install asks first.
    let zip = root.join("aurora.phypertheme");
    h.write(&zip).unwrap();
    let back = Hypertheme::open(&zip).unwrap();
    assert_eq!(back.assets["images/sky.png"], h.assets["images/sky.png"]);
    let e = stonedgem_hypertheme::install(&zip).unwrap_err();
    assert!(e.starts_with(NEEDS_POWER_CONFIRM), "{e}");
    assert_eq!(stonedgem_hypertheme::install_with(&zip, true).unwrap(), "aurora-glass.phypertheme");
    assert!(stonedgem_hypertheme::available().iter().any(|(f, n)| f == "aurora-glass.phypertheme" && n == "Aurora Glass"));
    assert!(stonedgem_hypertheme::load("aurora-glass.phypertheme").unwrap().is_power());

    // Theme Studio opens it and saves it back with its extra parts and pictures.
    let mut s = stonedgem_hypertheme::studio::Studio::new().unwrap();
    let (r, _) = s.message(serde_json::json!({ "kind": "open", "file": "aurora-glass.phypertheme" }));
    let r = r.unwrap();
    assert_eq!(r["power"], true);
    let graph = r["graph"].clone();
    assert!(graph["nodes"].as_array().unwrap().iter().any(|n| n["type"] == "theme"), "a graph built from its tokens");
    assert!(graph["extras"]["effect.qml"].is_string() && graph["extras"]["modules/palette.zxx"].is_string());
    let (r, _) = s.message(serde_json::json!({ "kind": "save", "graph": graph }));
    assert_eq!(r.unwrap()["file"], "aurora-glass.phypertheme");
    let saved = stonedgem_hypertheme::load("aurora-glass.phypertheme").unwrap();
    assert!(saved.files.contains_key("effect.qml") && saved.files.contains_key("graph.json"));
    assert!(saved.assets.contains_key("images/sky.png"), "pictures kept from the theme it opened");

    stonedgem_hypertheme::uninstall("aurora-glass.phypertheme").unwrap();
    assert!(stonedgem_hypertheme::uninstall("../x.hypertheme").is_err());
    let _ = std::fs::remove_dir_all(&root);
}
