//! Every part of a .hypertheme, merged in order, in light and dark.

use std::collections::BTreeMap;

use stonedgem_hypertheme::{contrast, Hypertheme};

fn files(list: &[(&str, &str)]) -> BTreeMap<String, String> {
    list.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn sample() -> BTreeMap<String, String> {
    files(&[
        ("hypertheme.toml", "[hypertheme]\nname = \"Sample\"\nauthor = \"Tests\"\n"),
        ("tokens.json", r##"{ "background": "#ffffff", "text": "#111111", "radius": 2, "dark": { "background": "#101010", "text": "#eeeeee" } }"##),
        ("style.rs", "pub const ACCENT: Color = rgb(255, 222, 89);\npub const ON_ACCENT: Color = hex(\"#000000\");\npub const RADIUS: f32 = 6.0;\npub mod dark { pub const ACCENT: Color = rgb(250, 200, 40); }\n"),
        ("theme.zsp", "func tokens(dark) { return { border: dark ? #ffffff : #000000, border_width: 3 } }\n"),
        ("theme.zxx", "fn tokens(dark) {\n    let base = #ff6b9d\n    return { danger: base, shadow_x: 5, shadow_y: 5, panels: { card: { fill: mix(base, #ffffff, 0.8) } } }\n}\n"),
        ("iced.toml", "[toolbar]\nfill = \"#ffde59\"\nborder_width = 3\n"),
        ("pages.css", ".ht-card { transform: rotate(-1deg); }\n"),
        ("start.tsx", "export default function Start() { return <main class=\"ht-start\"><h1>{theme.product}</h1><p>{theme.tokens.accent}</p></main>; }\n"),
        ("chrome.qml", "import QtQuick\nQtObject { property color accent: \"#ffde59\" }\n"),
        ("background.zsg", "fragment background(uv: vec2) -> color { return vec4(uv.x, 0.5, 0.2, 1.0) }\n"),
    ])
}

#[test]
fn all_parts_merge_in_order() {
    let h = Hypertheme::from_files(sample()).unwrap();
    let light = h.resolve(false);
    assert!(light.warnings.is_empty(), "{:?}", light.warnings);
    assert_eq!(light.color("background").unwrap(), "#ffffff"); // tokens.json
    assert_eq!(light.color("accent").unwrap(), "#ffde59"); // style.rs
    assert_eq!(light.number("radius"), Some(6.0)); // style.rs overrides tokens.json
    assert_eq!(light.color("border").unwrap(), "#000000"); // theme.zsp (Z#++)
    assert_eq!(light.color("danger").unwrap(), "#ff6b9d"); // theme.zxx (Zsharpend)
    assert_eq!(light.panel("card", "fill").unwrap(), "#ffe1eb"); // computed in Zsharpend
    assert_eq!(light.panel("toolbar", "border_width").unwrap(), 3); // iced.toml
    assert_eq!(light.color("surface").unwrap(), "#ffffff", "derived from background");

    let dark = h.resolve(true);
    assert_eq!(dark.color("background").unwrap(), "#101010");
    assert_eq!(dark.color("accent").unwrap(), "#fac828");
    assert_eq!(dark.color("border").unwrap(), "#ffffff");

    let css = light.css();
    assert!(css.contains("--ht-accent: #ffde59;") && css.contains("--ht-radius: 6px;"), "{css}");
    assert!(css.contains(".ht-toolbar { background: #ffde59;"), "{css}");

    let page = stonedgem_hypertheme::start_page(&h, &light, "StonedGem", Some("data:image/png;base64,AA")).unwrap();
    assert!(page.contains("render(h(__oynixWidget") && page.contains("rotate(-1deg)") && page.contains("url(data:image/png;base64,AA)"));
    let png = stonedgem_hypertheme::background_png(&h, 64, 32).unwrap().unwrap();
    assert!(png.starts_with(b"\x89PNG"));
}

#[test]
fn zip_round_trip_and_install() {
    let root = std::env::temp_dir().join(format!("hypertheme-{}", std::process::id()));
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    let h = Hypertheme::from_files(sample()).unwrap();
    let file = root.join("sample.hypertheme");
    h.write(&file).unwrap();
    let back = Hypertheme::open(&file).unwrap();
    assert_eq!(back.files, h.files);
    assert_eq!(stonedgem_hypertheme::install(&file).unwrap(), "sample.hypertheme");
    assert_eq!(stonedgem_hypertheme::installed(), vec![("sample.hypertheme".to_string(), "Sample".to_string())]);
}

#[test]
fn bad_themes_are_refused_and_contrast_is_checked() {
    let mut f = sample();
    f.insert("evil.sh".into(), "rm -rf /".into());
    assert!(Hypertheme::from_files(f).unwrap_err().contains("can't hold `evil.sh`"));
    let mut f = sample();
    f.insert("theme.zxx".into(), "fn tokens(dark) { return {".into());
    assert!(Hypertheme::from_files(f).unwrap_err().starts_with("theme.zxx:"));
    let mut f = sample();
    f.insert("background.zsg".into(), "fragment other(uv: vec2) -> color { return vec4(1.0, 1.0, 1.0, 1.0) }".into());
    assert!(Hypertheme::from_files(f).unwrap_err().contains("fragment background"));
    let low = Hypertheme::from_files(files(&[("hypertheme.toml", "[hypertheme]\nname = \"Low\"\n"), ("tokens.json", r##"{ "text": "#999999", "background": "#aaaaaa" }"##)])).unwrap();
    assert!(low.resolve(false).warnings.iter().any(|w| w.starts_with("text on background")));
    assert!((contrast("#000000", "#ffffff").unwrap() - 21.0).abs() < 0.01);
}
