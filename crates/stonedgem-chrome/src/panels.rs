//! Extension panels in Slint (`panel.slint`) — StonedGem's replacement for
//! OyNIx's QML panels (`graphics.qt`). The file is interpreted
//! (`slint-interpreter`) and drawn by Slint's software renderer into a
//! buffer the window shows beside the page; pointer and key events are
//! passed in. Callbacks the panel declares reach the extension's Z#++
//! script as `on panel (name, args)`, and the script sets the panel's
//! properties with `panel_set(name, value)`.
//!
//! The panel's component is the one named `Panel`, or else the last one
//! the file exports. Imports are limited to the extension's folder
//! (and Slint's own `std-widgets.slint`).

use std::cell::RefCell;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use i_slint_core::api::{LogicalPosition, PhysicalSize};
use i_slint_core::platform::{Platform, PointerEventButton, WindowAdapter, WindowEvent};
use i_slint_core::SharedString;
use i_slint_renderer_software::{MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType};
use serde_json::Value as Json;
use slint_interpreter::{ComponentDefinition, ComponentHandle, ComponentInstance, Compiler, Value};

pub const FILE: &str = "panel.slint";

thread_local! {
    static NEXT: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

struct Host {
    start: std::time::Instant,
}

impl Platform for Host {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, i_slint_core::platform::PlatformError> {
        let w = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        NEXT.with(|n| *n.borrow_mut() = Some(w.clone()));
        Ok(w)
    }
    fn duration_since_start(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

fn ensure_platform() {
    // Per thread; an error means it's already set for this one.
    let _ = i_slint_core::platform::set_platform(Box::new(Host { start: std::time::Instant::now() }));
}

/// Run a future that never waits on anything (the compiler's, with
/// files read synchronously).
fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

fn compile(src: &str, dir: &Path, dark: bool) -> Result<ComponentDefinition, String> {
    let mut c = Compiler::default();
    // Slint's Material widgets, in the window's scheme (docs/DESIGN.md).
    c.set_style(if dark { "material-dark" } else { "material-light" }.into());
    let root: PathBuf = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    c.set_file_loader(move |p: &Path| -> Pin<Box<dyn Future<Output = Option<std::io::Result<String>>>>> {
        let ok = p.canonicalize().map(|full| full.starts_with(&root)).unwrap_or(false);
        let r = if ok {
            None // inside the extension: load it normally
        } else {
            Some(Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("{}: panels can only import files in their extension", p.display()))))
        };
        Box::pin(async move { r })
    });
    let result = block_on(c.build_from_source(src.to_string(), dir.join(FILE)));
    let errors: Vec<String> = result
        .diagnostics()
        .filter(|d| d.level() == slint_interpreter::DiagnosticLevel::Error)
        .map(|d| {
            let (line, col) = d.line_column();
            format!("{line}:{col}: {}", d.message())
        })
        .collect();
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    result.component("Panel").or_else(|| result.components().last()).ok_or_else(|| "panel.slint exports no component".to_string())
}

/// For the extension registry: does `src` compile?
pub fn check(src: &str, dir: &Path) -> Result<(), String> {
    compile(src, dir, true).map(|_| ())
}

pub fn to_slint(j: &Json) -> Value {
    match j {
        Json::Null => Value::Void,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        Json::String(s) => Value::String(SharedString::from(s.as_str())),
        other => Value::String(SharedString::from(other.to_string().as_str())),
    }
}

pub fn from_slint(v: &Value) -> Json {
    match v {
        Value::Bool(b) => Json::Bool(*b),
        // Slint numbers are f64; whole ones (ints) go to scripts as integers.
        Value::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => serde_json::json!(*n as i64),
        Value::Number(n) => serde_json::json!(n),
        Value::String(s) => Json::String(s.to_string()),
        _ => Json::Null,
    }
}

type Events = Rc<RefCell<Vec<(String, Vec<Json>)>>>;

/// One open panel.
pub struct Panel {
    pub ext: String,
    pub title: String,
    instance: ComponentInstance,
    window: Rc<MinimalSoftwareWindow>,
    size: (u32, u32),
    buf: Vec<PremultipliedRgbaColor>,
    events: Events,
}

impl Panel {
    /// Open extension `ext`'s panel from its folder, `w`×`h` pixels, with
    /// Slint's widgets in the dark or light scheme.
    pub fn open(ext: &str, title: &str, dir: &Path, w: u32, h: u32, dark: bool) -> Result<Panel, String> {
        ensure_platform();
        let src = std::fs::read_to_string(dir.join(FILE)).map_err(|e| format!("{FILE}: {e}"))?;
        let def = compile(&src, dir, dark)?;
        NEXT.with(|n| n.borrow_mut().take());
        let instance = def.create().map_err(|e| e.to_string())?;
        let window = NEXT.with(|n| n.borrow_mut().take()).ok_or("no window for the panel")?;
        let events: Events = Rc::default();
        for name in def.callbacks() {
            let ev = events.clone();
            let n = name.clone();
            let _ = instance.set_callback(&name, move |args| {
                ev.borrow_mut().push((n.clone(), args.iter().map(from_slint).collect()));
                Value::Void
            });
        }
        instance.show().map_err(|e| e.to_string())?;
        let mut p = Panel { ext: ext.into(), title: title.into(), instance, window, size: (0, 0), buf: vec![], events };
        p.resize(w, h);
        Ok(p)
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.max(1), h.max(1));
        if self.size != (w, h) {
            self.size = (w, h);
            self.buf = vec![PremultipliedRgbaColor::default(); (w * h) as usize];
            self.window.set_size(PhysicalSize::new(w, h));
            self.window.request_redraw();
        }
    }

    /// Set a property the panel declares (`in property`).
    pub fn set(&self, prop: &str, value: &Json) -> Result<(), String> {
        self.instance.set_property(prop, to_slint(value)).map_err(|e| format!("{prop}: {e}"))
    }

    pub fn get(&self, prop: &str) -> Option<Json> {
        self.instance.get_property(prop).ok().map(|v| from_slint(&v))
    }

    /// Callbacks the panel invoked since the last call: (name, arguments).
    pub fn take_events(&self) -> Vec<(String, Vec<Json>)> {
        self.events.borrow_mut().drain(..).collect()
    }

    pub fn pointer_moved(&self, x: f32, y: f32) {
        self.window.window().dispatch_event(WindowEvent::PointerMoved { position: LogicalPosition::new(x, y) });
    }

    pub fn pointer_pressed(&self, x: f32, y: f32) {
        self.window.window().dispatch_event(WindowEvent::PointerPressed { position: LogicalPosition::new(x, y), button: PointerEventButton::Left });
    }

    pub fn pointer_released(&self, x: f32, y: f32) {
        self.window.window().dispatch_event(WindowEvent::PointerReleased { position: LogicalPosition::new(x, y), button: PointerEventButton::Left });
    }

    /// A click: press and release.
    pub fn click(&self, x: f32, y: f32) {
        self.pointer_moved(x, y);
        self.pointer_pressed(x, y);
        self.pointer_released(x, y);
    }

    pub fn scroll(&self, x: f32, y: f32, dx: f32, dy: f32) {
        self.window.window().dispatch_event(WindowEvent::PointerScrolled { position: LogicalPosition::new(x, y), delta_x: dx, delta_y: dy });
    }

    /// Type text (one key per character).
    pub fn text(&self, s: &str) {
        for c in s.chars() {
            let t = SharedString::from(c.to_string().as_str());
            self.window.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
            self.window.window().dispatch_event(WindowEvent::KeyReleased { text: t });
        }
    }

    /// Advance timers and animations and draw if anything changed.
    /// Returns straight RGBA when it drew.
    pub fn render(&mut self) -> Option<Vec<u8>> {
        i_slint_core::platform::update_timers_and_animations();
        let (w, _) = self.size;
        let buf = &mut self.buf;
        let drew = self.window.draw_if_needed(|r| {
            r.render(buf, w as usize);
        });
        if !drew {
            return None;
        }
        let mut out = Vec::with_capacity(self.buf.len() * 4);
        for p in &self.buf {
            let a = p.alpha as u32;
            let un = |c: u8| (c as u32 * 255 + a / 2).checked_div(a).map_or(0, |v| v.min(255) as u8);
            out.extend_from_slice(&[un(p.red), un(p.green), un(p.blue), p.alpha]);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_errors_are_reported() {
        let dir = std::env::temp_dir();
        assert!(check("export component Panel inherits Rectangle { width: 10px; }", &dir).is_ok());
        let e = check("export component Panel inherits Rectangle { width: 10px", &dir).unwrap_err();
        assert!(e.contains("1:"), "{e}");
        let e = check("import { X } from \"/etc/passwd\";\nexport component Panel inherits Rectangle {}", &dir).unwrap_err();
        assert!(!e.is_empty());
    }
}
