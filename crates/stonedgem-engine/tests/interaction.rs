//! What the browser window needs from the engine: frames to draw, and
//! clicks, typing, scrolling and history going to the page.

use std::time::Duration;

use stonedgem_engine::{Engine, Key, Load, Value};

const PAGE: &str = "data:text/html,<!DOCTYPE html><title>One</title>\
<style>html,body{margin:0;background:rgb(255,0,0)}\
button{position:absolute;left:10px;top:10px;width:100px;height:40px}\
input{position:absolute;left:10px;top:80px;width:200px;height:30px}\
.tall{height:5000px}</style>\
<button onclick=\"document.title='clicked'\">Go</button><input id=i><div class=tall></div>";

fn until(engine: &Engine, page: &stonedgem_engine::Page, js: &str) -> bool {
    for _ in 0..200 {
        if page.eval(engine, js, Duration::from_secs(5)) == Ok(Value::Boolean(true)) {
            return true;
        }
        engine.wait(Duration::from_millis(25), || false);
    }
    false
}

#[test]
fn frames_input_and_history() {
    let engine = Engine::headless(400, 300).expect("software rendering");
    let page = engine.open(PAGE, &[], &[]).unwrap();
    assert!(engine.wait(Duration::from_secs(30), || page.load() == Load::Complete));

    // A frame comes back at the engine's size, painted with the page.
    assert!(engine.wait(Duration::from_secs(10), || page.has_new_frame()));
    let f = page.frame(&engine).expect("a frame");
    assert_eq!((f.width, f.height), (400, 300));
    let [r, g, b, _] = f.pixel(300, 250);
    assert!(r > 200 && g < 50 && b < 50, "background should be red, got {r},{g},{b}");

    // Click the button.
    page.click(60.0, 30.0);
    assert!(until(&engine, &page, "document.title === 'clicked'"), "click didn't reach the button");

    // Click the field and type.
    page.click(100.0, 95.0);
    for k in [Key::Text("a".into()), Key::Text("b".into()), Key::Backspace] {
        page.key(&k);
    }
    assert!(until(&engine, &page, "document.getElementById('i').value === 'a'"), "typing didn't reach the field");

    // Scroll down.
    page.scroll(200.0, 200.0, 0.0, 300.0);
    assert!(until(&engine, &page, "window.scrollY > 0"), "scrolling didn't move the page");

    // Resize: frames follow.
    engine.resize(&page, 320, 200);
    engine.wait(Duration::from_millis(300), || false);
    let f = page.frame(&engine).expect("a frame after resize");
    assert_eq!((f.width, f.height), (320, 200));

    // History.
    page.navigate("data:text/html,<title>Two</title>").unwrap();
    assert!(engine.wait(Duration::from_secs(20), || page.title().as_deref() == Some("Two")));
    assert!(engine.wait(Duration::from_secs(5), || page.can_go_back()));
    page.back();
    assert!(engine.wait(Duration::from_secs(20), || page.title().as_deref() == Some("clicked") || page.title().as_deref() == Some("One")));
    assert!(page.take_changed());

    // Zoom: CSS pixels get bigger, so the viewport gets narrower.
    let before = page.eval(&engine, "window.innerWidth", Duration::from_secs(5));
    page.set_zoom(2.0);
    assert!((page.zoom() - 2.0).abs() < 1e-3);
    let mut zoomed = false;
    for _ in 0..100 {
        let _ = page.frame(&engine); // zoom applies when the page paints
        if page.eval(&engine, "window.devicePixelRatio >= 2 && window.innerWidth <= 200", Duration::from_secs(5)) == Ok(Value::Boolean(true)) {
            zoomed = true;
            break;
        }
        engine.wait(Duration::from_millis(25), || false);
    }
    let now = page.eval(&engine, "[window.devicePixelRatio, window.innerWidth, document.documentElement.clientWidth]", Duration::from_secs(5));
    assert!(zoomed, "zoom didn't change the page (innerWidth was {before:?}, now {now:?}, zoom {})", page.zoom());
    page.set_zoom(1.0);

    page.hide();
    switching_pages_switches_frames(&engine);
}

/// Called from `frames_input_and_history`: Servo starts once per process.
fn switching_pages_switches_frames(engine: &Engine) {
    let red = engine.open("data:text/html,<style>html{background:rgb(255,0,0)}</style>", &[], &[]).unwrap();
    let blue = engine.open("data:text/html,<style>html{background:rgb(0,0,255)}</style>", &[], &[]).unwrap();
    assert!(engine.wait(Duration::from_secs(30), || red.load() == Load::Complete && blue.load() == Load::Complete));
    let colour = |page: &stonedgem_engine::Page| {
        engine.wait(Duration::from_millis(300), || false);
        page.frame(engine).unwrap().pixel(100, 75)
    };
    red.hide();
    blue.show();
    let [r, _, b, _] = colour(&blue);
    assert!(b > 200 && r < 50, "blue page shown, got {r},{b}");
    blue.hide();
    red.show();
    let [r, _, b, _] = colour(&red);
    assert!(r > 200 && b < 50, "red page shown, got {r},{b}");
}
