//! Find in page, run for real in the engine.

use std::time::Duration;

use stonedgem_chrome::find;
use stonedgem_engine::{Engine, Load, Value};

fn ask(engine: &Engine, page: &stonedgem_engine::Page, q: &str, step: i32) -> Option<(usize, usize)> {
    match page.eval(engine, &find::script(q, step), Duration::from_secs(5)) {
        Ok(Value::String(s)) => find::parse(&s),
        other => panic!("find answered {other:?}"),
    }
}

#[test]
fn finds_steps_and_clears() {
    let engine = Engine::headless(400, 300).expect("software rendering");
    let page = engine
        .open("data:text/html,<p>Servo renders. SERVO paints.</p><script>var servo=1</script><p>servo again</p>", &[], &[])
        .unwrap();
    assert!(engine.wait(Duration::from_secs(30), || page.load() == Load::Complete));
    let t = Duration::from_secs(5);

    // Case-insensitive, skips <script>.
    assert_eq!(ask(&engine, &page, "servo", 1), Some((1, 3)));
    assert_eq!(ask(&engine, &page, "servo", 1), Some((2, 3)));
    assert_eq!(ask(&engine, &page, "servo", 1), Some((3, 3)));
    assert_eq!(ask(&engine, &page, "servo", 1), Some((1, 3)), "wraps around");
    assert_eq!(ask(&engine, &page, "servo", -1), Some((3, 3)), "and backwards");
    assert_eq!(page.eval(&engine, "document.querySelectorAll('mark').length", t), Ok(Value::Number(3.0)));

    // A new query replaces the old highlights.
    assert_eq!(ask(&engine, &page, "paints", 1), Some((1, 1)));
    assert_eq!(page.eval(&engine, "document.querySelectorAll('mark').length", t), Ok(Value::Number(1.0)));
    assert_eq!(ask(&engine, &page, "nothing here", 1), Some((0, 0)));

    // Closing (empty query) leaves the page as it was.
    assert_eq!(ask(&engine, &page, "", 1), Some((0, 0)));
    assert_eq!(page.eval(&engine, "document.querySelectorAll('mark').length", t), Ok(Value::Number(0.0)));
    assert_eq!(
        page.eval(&engine, "document.body.textContent", t),
        Ok(Value::String("Servo renders. SERVO paints.var servo=1servo again".into()))
    );
}
