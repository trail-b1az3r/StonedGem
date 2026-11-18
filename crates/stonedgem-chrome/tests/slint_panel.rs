//! An extension's Slint panel: installed (checked), drawn by the software
//! renderer, clicked; the click reaches the extension's Z#++ script, whose
//! `panel_set` updates the panel.

use stonedgem_chrome::panels::{self, Panel};
use stonedgem_chrome::zs_hooks::{Action, Scripts};
use stonedgem_extensions::registry;

const PANEL: &str = r#"
import { Button } from "std-widgets.slint";
export component Panel inherits Window {
    in property <string> label: "Ready";
    in property <int> count: 0;
    callback clicked(int);
    background: #204080;
    VerticalLayout {
        padding: 12px;
        Text { text: label; color: white; font-size: 18px; }
        TouchArea {
            height: 60px;
            clicked => { root.clicked(count + 1); }
            Rectangle { background: count > 0 ? #30c060 : #c03030; }
        }
    }
}
"#;

#[test]
fn panel_draws_and_talks_to_its_script() {
    let root = std::env::temp_dir().join(format!("stonedgem-slint-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    stonedgem_extensions::host::set_slint_check(panels::check);

    let src = root.join("src/counter");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("oynix.json"), r#"{"name": "Counter", "version": "1.0", "matches": []}"#).unwrap();
    std::fs::write(src.join("code.ts"), "export {};").unwrap();
    std::fs::write(src.join("content.css"), "").unwrap();
    std::fs::write(src.join("panel.slint"), PANEL).unwrap();
    std::fs::write(
        src.join("script.zsp"),
        "on panel (name, args) {\n    if name == \"clicked\" {\n        panel_set(\"count\", args[0])\n        panel_set(\"label\", \"Clicked ${args[0]}\")\n    }\n}\n",
    )
    .unwrap();

    let mut list = vec![];
    let e = registry::install(&src, &mut list).expect("install");
    assert!(e.has_slint_panel && e.has_script);

    // A broken panel is refused at install with the line.
    let bad = root.join("src/bad");
    std::fs::create_dir_all(&bad).unwrap();
    for (f, t) in [("oynix.json", r#"{"name": "Bad", "matches": []}"#), ("code.ts", "export {};"), ("content.css", ""), ("panel.slint", "export component Panel {")] {
        std::fs::write(bad.join(f), t).unwrap();
    }
    let err = registry::install(&bad, &mut list).unwrap_err();
    assert!(err.contains("panel.slint"), "{err}");

    let mut scripts = Scripts::load(&list);
    assert_eq!(scripts.len(), 1, "{:?}", scripts.errors);

    let mut p = Panel::open(&e.id, &e.name, std::path::Path::new(&e.path), 200, 120, true).expect("open panel");
    let px = p.render().expect("first frame");
    let at = |px: &[u8], x: u32, y: u32| {
        let i = ((y * 200 + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    };
    assert_eq!(at(&px, 2, 2), [0x20, 0x40, 0x80, 255], "panel background");
    let button = at(&px, 100, 80);
    assert!(button[0] > 150 && button[1] < 80, "red before the click: {button:?}");

    p.click(100.0, 80.0);
    let events = p.take_events();
    assert_eq!(events, vec![("clicked".to_string(), vec![serde_json::json!(1)])]);
    for (name, args) in events {
        let (r, actions) = scripts.panel(&e.id, &name, args.iter().map(stonedgem_chrome::bridge::from_json).collect()).unwrap();
        r.unwrap();
        for a in actions {
            if let Action::PanelSet(k, v) = a {
                p.set(&k, &v).unwrap();
            }
        }
    }
    assert_eq!(p.get("label"), Some(serde_json::json!("Clicked 1")));
    let px = p.render().expect("redrawn after the change");
    let button = at(&px, 100, 80);
    assert!(button[1] > 150 && button[0] < 100, "green after the click: {button:?}");
}
