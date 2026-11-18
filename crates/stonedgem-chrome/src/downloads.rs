//! Downloads. Servo 0.8 doesn't download files itself, so the window does:
//! each download runs on its own thread with the same HTTP client OyNIx's
//! updater uses, writing to `<name>.part` and renaming when complete.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Running,
    Done,
    Failed(String),
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct Download {
    pub id: u64,
    pub url: String,
    pub file: PathBuf,
    pub received: u64,
    pub total: Option<u64>,
    pub state: State,
}

impl Download {
    /// "1.2 MB of 3.4 MB", "820 KB", …
    pub fn progress(&self) -> String {
        match self.total {
            Some(t) if self.state == State::Running => format!("{} of {}", size(self.received), size(t)),
            _ => size(self.received),
        }
    }
}

/// A byte count for people.
pub fn size(n: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if n < 1000 {
        return format!("{n} B");
    }
    let mut v = n as f64 / 1000.0;
    let mut u = 0;
    while v >= 1000.0 && u < UNITS.len() - 1 {
        v /= 1000.0;
        u += 1;
    }
    format!("{v:.1} {}", UNITS[u])
}

/// The file name for `url`: the server's `Content-Disposition` name if
/// given, else the last path segment, else "download". Never a path.
pub fn file_name(url: &str, disposition: Option<&str>) -> String {
    let from_header = disposition.and_then(|d| {
        d.split(';').map(str::trim).find_map(|p| p.strip_prefix("filename=")).map(|n| n.trim_matches('"').to_string())
    });
    let from_url = || {
        let path = url.split(['?', '#']).next().unwrap_or("");
        path.rsplit('/').next().filter(|s| !s.is_empty()).map(|s| s.replace("%20", " "))
    };
    let name = from_header.or_else(from_url).unwrap_or_else(|| "download".into());
    let clean: String = name.chars().map(|c| if c == '/' || c == '\\' || c.is_control() { '_' } else { c }).collect();
    let clean = clean.trim_start_matches('.').trim();
    if clean.is_empty() { "download".into() } else { clean.to_string() }
}

/// `dir/name`, or `dir/name (1).ext`, `(2)`… if taken.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() && !dir.join(format!("{name}.part")).exists() {
        return p;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    (1..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists() && !p.with_extension(format!("{}part", p.extension().map(|e| format!("{}.", e.to_string_lossy())).unwrap_or_default())).exists())
        .unwrap()
}

/// Per download: its id and the flag that cancels it.
type CancelFlags = Arc<Mutex<Vec<(u64, Arc<AtomicBool>)>>>;
/// Updates a download's entry in the shared list.
type Update<'a> = &'a dyn Fn(&dyn Fn(&mut Download));

#[derive(Default)]
pub struct Downloads {
    list: Arc<Mutex<Vec<Download>>>,
    cancel: CancelFlags,
    next: u64,
}

impl Downloads {
    /// Start downloading `url` into `dir`. Returns its id.
    pub fn start(&mut self, url: &str, dir: &Path) -> u64 {
        self.next += 1;
        let id = self.next;
        let stop = Arc::new(AtomicBool::new(false));
        self.cancel.lock().unwrap().push((id, stop.clone()));
        self.list.lock().unwrap().push(Download { id, url: url.into(), file: PathBuf::new(), received: 0, total: None, state: State::Running });
        let (list, url, dir) = (self.list.clone(), url.to_string(), dir.to_path_buf());
        std::thread::Builder::new()
            .name("stonedgem-download".into())
            .spawn(move || {
                let set = |f: &dyn Fn(&mut Download)| {
                    if let Some(d) = list.lock().unwrap().iter_mut().find(|d| d.id == id) {
                        f(d);
                    }
                };
                let result = fetch(&url, &dir, &stop, &set);
                set(&|d| {
                    d.state = match (&result, stop.load(Ordering::Relaxed)) {
                        (_, true) => State::Cancelled,
                        (Ok(()), _) => State::Done,
                        (Err(e), _) => State::Failed(e.clone()),
                    }
                });
            })
            .ok();
        id
    }

    pub fn cancel(&self, id: u64) {
        if let Some((_, s)) = self.cancel.lock().unwrap().iter().find(|(i, _)| *i == id) {
            s.store(true, Ordering::Relaxed);
        }
    }

    /// Forget finished downloads (the files stay).
    pub fn clear_finished(&self) {
        self.list.lock().unwrap().retain(|d| d.state == State::Running);
    }

    pub fn snapshot(&self) -> Vec<Download> {
        self.list.lock().unwrap().clone()
    }

    pub fn running(&self) -> usize {
        self.list.lock().unwrap().iter().filter(|d| d.state == State::Running).count()
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
        // A browser's user agent: some sites refuse unknown clients.
        .user_agent(concat!("Mozilla/5.0 (X11; Linux x86_64) StonedGem/", env!("CARGO_PKG_VERSION")))
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .build()
        .into()
}

fn fetch(url: &str, dir: &Path, stop: &AtomicBool, set: Update) -> Result<(), String> {
    let mut resp = agent().get(url).call().map_err(|e| match e {
        ureq::Error::StatusCode(429) => "the site says too many requests; try again later".to_string(),
        ureq::Error::StatusCode(403) => "the site refused the download (403)".to_string(),
        ureq::Error::StatusCode(404) => "the file isn't there any more (404)".to_string(),
        ureq::Error::StatusCode(c) => format!("the site answered {c}"),
        e => e.to_string(),
    })?;
    let disposition = resp.headers().get("content-disposition").and_then(|v| v.to_str().ok()).map(str::to_string);
    let total = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let file = unique_path(dir, &file_name(url, disposition.as_deref()));
    let part = PathBuf::from(format!("{}.part", file.display()));
    set(&|d| {
        d.file = file.clone();
        d.total = total;
    });
    let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut body = resp.body_mut().as_reader();
    let mut buf = vec![0u8; 64 * 1024];
    let mut got = 0u64;
    loop {
        if stop.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&part);
            return Err("cancelled".into());
        }
        let n = body.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        got += n as u64;
        set(&|d| d.received = got);
    }
    out.flush().map_err(|e| e.to_string())?;
    std::fs::rename(&part, &file).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn names() {
        assert_eq!(file_name("https://x.org/a/report.pdf?dl=1", None), "report.pdf");
        assert_eq!(file_name("https://x.org/", None), "download");
        assert_eq!(file_name("https://x.org/get", Some("attachment; filename=\"Q3 data.csv\"")), "Q3 data.csv");
        assert_eq!(file_name("https://x.org/get", Some("attachment; filename=../../etc/passwd")), "_.._etc_passwd");
        assert_eq!(size(999), "999 B");
        assert_eq!(size(1_500_000), "1.5 MB");
    }

    #[test]
    fn unique_names() {
        let d = std::env::temp_dir().join(format!("stonedgem-dl-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(unique_path(&d, "a.txt"), d.join("a.txt"));
        std::fs::write(d.join("a.txt"), "").unwrap();
        assert_eq!(unique_path(&d, "a.txt"), d.join("a (1).txt"));
        std::fs::write(d.join("a (1).txt"), "").unwrap();
        assert_eq!(unique_path(&d, "a.txt"), d.join("a (2).txt"));
        assert_eq!(unique_path(&d, "noext"), d.join("noext"));
    }

    #[test]
    fn downloads_from_a_server() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let body = vec![7u8; 300_000];
        let b2 = body.clone();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut req = [0u8; 2048];
                let _ = s.read(&mut req);
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Disposition: attachment; filename=\"data.bin\"\r\nConnection: close\r\n\r\n",
                    b2.len()
                );
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(&b2);
            }
        });
        let d = std::env::temp_dir().join(format!("stonedgem-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut dl = Downloads::default();
        dl.start(&format!("http://{addr}/x"), &d);
        let t = std::time::Instant::now();
        while dl.running() > 0 && t.elapsed().as_secs() < 20 {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let s = dl.snapshot();
        assert_eq!(s[0].state, State::Done, "{s:?}");
        assert_eq!(s[0].file, d.join("data.bin"));
        assert_eq!(s[0].total, Some(300_000));
        assert_eq!(std::fs::read(d.join("data.bin")).unwrap(), body);
        assert!(!d.join("data.bin.part").exists());
    }
}
