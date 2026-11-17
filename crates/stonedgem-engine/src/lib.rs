//! StonedGem's web engine: [Servo](https://servo.org) behind a small API of
//! StonedGem's own, so the browser chrome never touches Servo types and the
//! engine can be swapped.
//!
//! What the spike covers (see `docs/ENGINE-SPIKE.md`):
//!
//! * pages render off-screen ([`Engine::headless`]) — a window comes with
//!   the Iced chrome in phase 5;
//! * every HTTP(S) request goes through `stonedgem-blocker` first, and a
//!   blocked one fails like a network error, as in OyNIx;
//! * extensions' content scripts and styles are injected as user scripts
//!   and user style sheets, and element-hiding rules become a style sheet.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use servo::{
    EventLoopWaker, JSValue, LoadStatus, Preferences, RenderingContext, Servo, ServoBuilder, SoftwareRenderingContext,
    UserContentManager, UserScript, UserStyleSheet, WebResourceLoad, WebResourceResponse, WebView, WebViewBuilder,
    WebViewDelegate,
};

pub use servo::JSValue as Value;

/// Whether a page has finished loading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Load {
    Loading,
    Complete,
}

/// The resource-type numbers `stonedgem-blocker` uses (they match
/// QtWebEngine's, which OyNIx's filter code was written against).
fn resource_type(d: content_security_policy::Destination, main_frame: bool) -> i32 {
    use content_security_policy::Destination as D;
    match d {
        D::Document if main_frame => 0,
        D::Document | D::Frame | D::IFrame => 1,
        D::Style => 2,
        D::Script | D::Worker | D::SharedWorker | D::ServiceWorker | D::AudioWorklet | D::PaintWorklet | D::Xslt => 3,
        D::Image => 4,
        D::Font => 5,
        D::Object | D::Embed => 7,
        D::Audio | D::Video | D::Track => 8,
        D::None | D::Json => 13,
        D::Report => 14,
        _ => 255,
    }
}

#[derive(Clone)]
struct Waker;
impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Waker)
    }
    fn wake(&self) {}
}

/// Per-page state the delegate fills in.
#[derive(Default)]
struct PageState {
    load: Cell<Option<LoadStatus>>,
    blocked: RefCell<Vec<String>>,
    console: RefCell<Vec<String>>,
}

struct Delegate(Rc<PageState>);

impl WebViewDelegate for Delegate {
    fn load_web_resource(&self, webview: WebView, load: WebResourceLoad) {
        let req = load.request();
        let url = req.url.as_str().to_string();
        let first_party = if req.is_for_main_frame {
            url.clone()
        } else {
            webview.url().map(|u| u.to_string()).unwrap_or_default()
        };
        let kind = resource_type(req.destination, req.is_for_main_frame);
        if stonedgem_blocker::blocker::should_block(&url, &first_party, kind) {
            self.0.blocked.borrow_mut().push(url);
            if let Ok(u) = url::Url::parse(req.url.as_str()) {
                // Fail the load like a network error: no body reaches the page.
                load.intercept(WebResourceResponse::new(u).status_code(http::StatusCode::FORBIDDEN)).cancel();
            }
        }
        // Not intercepted: Servo loads it normally when `load` drops.
    }

    fn notify_load_status_changed(&self, _webview: WebView, status: LoadStatus) {
        self.0.load.set(Some(status));
    }

    fn show_console_message(&self, _webview: WebView, _level: servo::ConsoleLogLevel, message: String) {
        self.0.console.borrow_mut().push(message);
    }
}

/// One Servo instance. Servo runs on the thread that created it; spin it
/// with [`Engine::spin`] or [`Engine::wait`].
pub struct Engine {
    servo: Servo,
    context: Rc<SoftwareRenderingContext>,
}

impl Engine {
    /// An engine that renders into memory (no window), `width`×`height`.
    pub fn headless(width: u32, height: u32) -> Result<Self, String> {
        let context = Rc::new(
            SoftwareRenderingContext::new(dpi::PhysicalSize { width, height })
                .map_err(|e| format!("software rendering: {e:?}"))?,
        );
        context.make_current().map_err(|e| format!("software rendering: {e:?}"))?;
        let servo = ServoBuilder::default().preferences(Preferences::default()).event_loop_waker(Box::new(Waker)).build();
        Ok(Engine { servo, context })
    }

    /// Run Servo's event loop once.
    pub fn spin(&self) {
        self.servo.spin_event_loop();
    }

    /// Spin until `done` returns true or `timeout` passes. Returns `done()`.
    pub fn wait(&self, timeout: Duration, done: impl Fn() -> bool) -> bool {
        let end = Instant::now() + timeout;
        while !done() {
            if Instant::now() > end {
                return false;
            }
            self.spin();
            std::thread::sleep(Duration::from_millis(2));
        }
        true
    }

    /// Open `url` in a new page with these content scripts and styles
    /// (extensions'), plus the blocker's element-hiding rules for its host.
    pub fn open(&self, url: &str, scripts: &[String], styles: &[String]) -> Result<Page, String> {
        let parsed = url::Url::parse(url).map_err(|e| format!("{url}: {e}"))?;
        let ucm = Rc::new(UserContentManager::new(&self.servo));
        for s in scripts {
            ucm.add_script(Rc::new(UserScript::from(s.as_str())));
        }
        let mut css: Vec<String> = styles.to_vec();
        let hide = stonedgem_blocker::blocker::cosmetic_selectors(&stonedgem_url::host_of(url));
        if !hide.is_empty() {
            css.push(format!("{} {{ display: none !important; }}", hide.join(",\n")));
        }
        for c in css {
            ucm.add_stylesheet(Rc::new(UserStyleSheet::new(c, parsed.clone())));
        }
        let state = Rc::new(PageState::default());
        let rc: Rc<dyn RenderingContext> = self.context.clone();
        let webview = WebViewBuilder::new(&self.servo, rc)
            .delegate(Rc::new(Delegate(state.clone())))
            .user_content_manager(ucm)
            .url(parsed)
            .build();
        Ok(Page { webview, state })
    }
}

/// A page (Servo `WebView`).
pub struct Page {
    webview: WebView,
    state: Rc<PageState>,
}

impl Page {
    pub fn load(&self) -> Load {
        match self.state.load.get() {
            Some(LoadStatus::Complete) => Load::Complete,
            _ => Load::Loading,
        }
    }

    pub fn url(&self) -> Option<String> {
        self.webview.url().map(|u| u.to_string())
    }

    pub fn title(&self) -> Option<String> {
        self.webview.page_title()
    }

    /// Requests the blocker stopped on this page, in order.
    pub fn blocked(&self) -> Vec<String> {
        self.state.blocked.borrow().clone()
    }

    /// Console messages the page logged.
    pub fn console(&self) -> Vec<String> {
        self.state.console.borrow().clone()
    }

    pub fn navigate(&self, url: &str) -> Result<(), String> {
        self.state.load.set(None);
        self.webview.load(url::Url::parse(url).map_err(|e| format!("{url}: {e}"))?);
        Ok(())
    }

    /// Run `script` in the page and wait (spinning `engine`) for its value.
    pub fn eval(&self, engine: &Engine, script: &str, timeout: Duration) -> Result<JSValue, String> {
        let out: Rc<RefCell<Option<Result<JSValue, String>>>> = Rc::default();
        let o = out.clone();
        self.webview.evaluate_javascript(script, move |r| {
            *o.borrow_mut() = Some(r.map_err(|e| format!("{e:?}")));
        });
        if !engine.wait(timeout, || out.borrow().is_some()) {
            return Err("timed out".into());
        }
        out.borrow_mut().take().unwrap()
    }
}
