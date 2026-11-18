//! The page↔browser bridge. Servo has no QWebChannel, so pages get a
//! stand-in: `post(...)` queues a message in the page, the window collects
//! the queue every few frames ([`DRAIN_JS`]) and answers by calling back
//! into the page. The same bridge serves:
//!
//! * `network.ts` (power extensions): OyNIx's `oynix.network` API talks to
//!   a `QWebChannel` object; [`webchannel_shim`] is that object here.
//! * the `chrome.*` / `browser.*` subset for extension content scripts
//!   ([`chrome_shim`]): `runtime.id`, `runtime.getManifest()`,
//!   `runtime.sendMessage()` (answered by the extension's Z#++
//!   `on message` handler), `storage.local` (`get`, `set`, `remove`,
//!   `clear`; saved per extension in the profile), `tabs.create({url})`.

use std::collections::BTreeMap;

use serde_json::{json, Value as Json};
use stonedgem_zsharp::Value;

/// Collect what pages queued: a JSON array of `[extension id, kind, json]`.
pub const DRAIN_JS: &str = "JSON.stringify((window.__sgOut || []).splice(0))";

/// Stands in for Qt's `qwebchannel.js`: `oynixExtensions.post(id, kind, json)`.
pub fn webchannel_shim() -> String {
    "var qt = window.qt || { webChannelTransport: {} }; window.__sgOut = window.__sgOut || [];\n\
     function QWebChannel(t, cb) { cb({ objects: { oynixExtensions: { post: function (id, kind, json) { window.__sgOut.push([id, kind, json]); } } } }); }\n"
        .to_string()
}

/// A secret per extension for this run of the browser, kept in its
/// shim's closure: messages without it are ignored, so a page can't call
/// an extension's `chrome.*` by queueing messages under its id.
pub fn ext_secret(id: &str) -> String {
    use std::hash::{BuildHasher, Hasher};
    static KEYS: std::sync::OnceLock<[std::collections::hash_map::RandomState; 2]> = std::sync::OnceLock::new();
    let keys = KEYS.get_or_init(Default::default);
    let part = |k: &std::collections::hash_map::RandomState| {
        let mut h = k.build_hasher();
        h.write(id.as_bytes());
        h.finish()
    };
    format!("{:016x}{:016x}", part(&keys[0]), part(&keys[1]))
}

/// The `chrome` / `browser` subset for content scripts of extension `id`.
pub fn chrome_shim(id: &str, manifest: &Json) -> String {
    format!(
        r#"(function () {{
  var id = {id}, secret = {secret}, manifest = {manifest}, seq = 0, pending = {{}};
  window.__sgOut = window.__sgOut || [];
  window.__sgChrome = window.__sgChrome || {{}};
  window.__sgChrome[id] = function (req, value, error) {{
    var p = pending[req]; if (!p) return; delete pending[req];
    if (error) p.reject(new Error(error)); else p.resolve(value);
  }};
  function call(op, args, cb) {{
    var req = 'c' + (++seq);
    var pr = new Promise(function (resolve, reject) {{ pending[req] = {{ resolve: resolve, reject: reject }}; }});
    window.__sgOut.push([id, 'chrome', JSON.stringify(Object.assign({{ op: op, req: req, secret: secret }}, args || {{}}))]);
    if (typeof cb === 'function') pr.then(cb, function () {{ cb(undefined); }});
    return pr;
  }}
  var api = {{
    runtime: {{
      id: id,
      getManifest: function () {{ return JSON.parse(JSON.stringify(manifest)); }},
      sendMessage: function (msg, cb) {{ return call('sendMessage', {{ message: msg }}, cb); }}
    }},
    storage: {{ local: {{
      get: function (keys, cb) {{ return call('storage.get', {{ keys: keys === undefined ? null : keys }}, cb); }},
      set: function (items, cb) {{ return call('storage.set', {{ items: items }}, cb); }},
      remove: function (keys, cb) {{ return call('storage.remove', {{ keys: keys }}, cb); }},
      clear: function (cb) {{ return call('storage.clear', {{}}, cb); }}
    }} }},
    tabs: {{ create: function (o, cb) {{ return call('tabs.create', {{ url: String((o || {{}}).url || '') }}, cb); }} }}
  }};
  window.chrome = window.chrome || {{}};
  for (var k in api) window.chrome[k] = api[k];
  window.browser = window.chrome;
}})();
"#,
        id = serde_json::to_string(id).unwrap_or_default(),
        secret = serde_json::to_string(&ext_secret(id)).unwrap_or_default(),
        manifest = manifest
    )
}

/// The answer to a `chrome` request, as JavaScript for the page.
pub fn chrome_reply(id: &str, req: &str, result: Result<Json, String>) -> String {
    let (v, e) = match result {
        Ok(v) => (v, Json::Null),
        Err(e) => (Json::Null, Json::String(e)),
    };
    format!(
        "window.__sgChrome && window.__sgChrome[{}] && window.__sgChrome[{}]({}, {}, {})",
        json!(id),
        json!(id),
        json!(req),
        v,
        e
    )
}

/// The answer to an `oynix.network` request.
pub fn net_reply(req: &str, result: Result<Json, String>) -> String {
    let (v, e) = match result {
        Ok(v) => (v, Json::Null),
        Err(e) => (Json::Null, Json::String(e)),
    };
    format!("window.__oynixNet && window.__oynixNet.result({}, {}, {})", json!(req), v, e)
}

/// Deliver observed requests to a `network.ts` page.
pub fn net_requests(list: &Json) -> String {
    format!("window.__oynixNet && window.__oynixNet.requests({list})")
}

/// Parse what [`DRAIN_JS`] returned.
pub fn parse_drain(s: &str) -> Vec<(String, String, Json)> {
    let Ok(Json::Array(items)) = serde_json::from_str::<Json>(s) else { return vec![] };
    items
        .into_iter()
        .filter_map(|it| {
            let a = it.as_array()?;
            let id = a.first()?.as_str()?.to_string();
            let kind = a.get(1)?.as_str()?.to_string();
            let body: Json = serde_json::from_str(a.get(2)?.as_str()?).ok()?;
            Some((id, kind, body))
        })
        .collect()
}

// ── storage.local ──────────────────────────────────────────────────────

fn storage_file(ext: &str) -> std::path::PathBuf {
    let safe: String = ext.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.').collect();
    stonedgem_profile::data_dir().join("ext-storage").join(format!("{safe}.json"))
}

pub fn storage_load(ext: &str) -> BTreeMap<String, Json> {
    std::fs::read(storage_file(ext)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn storage_save(ext: &str, m: &BTreeMap<String, Json>) -> Result<(), String> {
    let f = storage_file(ext);
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let b = serde_json::to_vec(m).map_err(|e| e.to_string())?;
    if b.len() > 5 << 20 {
        return Err("storage.local is full (5 MB)".into());
    }
    stonedgem_profile::write_atomic(&f, &b).map_err(|e| e.to_string())
}

/// `chrome.storage.local.*` for extension `ext`.
pub fn storage(ext: &str, op: &str, body: &Json) -> Result<Json, String> {
    let mut m = storage_load(ext);
    let keys = |k: &Json| -> Vec<String> {
        match k {
            Json::String(s) => vec![s.clone()],
            Json::Array(a) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
            Json::Object(o) => o.keys().cloned().collect(),
            _ => vec![],
        }
    };
    match op {
        "storage.get" => {
            let k = &body["keys"];
            let mut out = serde_json::Map::new();
            match k {
                Json::Null => out.extend(m),
                Json::Object(defaults) => {
                    for (key, d) in defaults {
                        out.insert(key.clone(), m.get(key).cloned().unwrap_or_else(|| d.clone()));
                    }
                }
                other => {
                    for key in keys(other) {
                        if let Some(v) = m.get(&key) {
                            out.insert(key, v.clone());
                        }
                    }
                }
            }
            Ok(Json::Object(out))
        }
        "storage.set" => {
            if let Json::Object(items) = &body["items"] {
                for (k, v) in items {
                    m.insert(k.clone(), v.clone());
                }
            }
            storage_save(ext, &m)?;
            Ok(Json::Null)
        }
        "storage.remove" => {
            for k in keys(&body["keys"]) {
                m.remove(&k);
            }
            storage_save(ext, &m)?;
            Ok(Json::Null)
        }
        "storage.clear" => {
            storage_save(ext, &BTreeMap::new())?;
            Ok(Json::Null)
        }
        _ => Err(format!("{op} isn't supported")),
    }
}

// ── Z#++ ↔ JSON ────────────────────────────────────────────────────────

pub fn to_json(v: &Value) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Int(i) => json!(i),
        Value::Float(f) => json!(f),
        Value::Str(s) => Json::String(s.to_string()),
        Value::List(l) => Json::Array(l.borrow().iter().map(to_json).collect()),
        Value::Map(m) => Json::Object(m.borrow().iter().map(|(k, v)| (k.clone(), to_json(v))).collect()),
        Value::Struct(s) => Json::Object(s.borrow().fields.iter().map(|(k, v)| (k.clone(), to_json(v))).collect()),
        other => Json::String(other.to_string()),
    }
}

pub fn from_json(j: &Json) -> Value {
    match j {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Value::Int(i),
            None => Value::Float(n.as_f64().unwrap_or(0.0)),
        },
        Json::String(s) => Value::str(s),
        Json::Array(a) => Value::list(a.iter().map(from_json).collect()),
        Json::Object(o) => Value::map(o.iter().map(|(k, v)| (k.clone(), from_json(v))).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_parsing_and_json() {
        let d = parse_drain(r#"[["ext1","chrome","{\"op\":\"storage.get\",\"req\":\"c1\"}"],["bad"]]"#);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].2["op"], "storage.get");
        let v = from_json(&json!({"a": [1, 2.5, "x"], "b": null}));
        assert_eq!(to_json(&v), json!({"a": [1, 2.5, "x"], "b": null}));
    }

    #[test]
    fn storage_round_trip() {
        std::env::set_var("XDG_DATA_HOME", std::env::temp_dir().join(format!("sg-bridge-{}", std::process::id())));
        let ext = "storage-test";
        storage(ext, "storage.clear", &json!({})).unwrap();
        storage(ext, "storage.set", &json!({"items": {"a": 1, "b": {"c": true}}})).unwrap();
        assert_eq!(storage(ext, "storage.get", &json!({"keys": "a"})).unwrap(), json!({"a": 1}));
        assert_eq!(storage(ext, "storage.get", &json!({"keys": {"a": 0, "z": 9}})).unwrap(), json!({"a": 1, "z": 9}));
        storage(ext, "storage.remove", &json!({"keys": ["a"]})).unwrap();
        assert_eq!(storage(ext, "storage.get", &json!({"keys": null})).unwrap(), json!({"b": {"c": true}}));
    }
}
