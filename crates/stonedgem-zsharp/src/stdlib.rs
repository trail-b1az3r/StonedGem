//! Built-in functions. Every function also works as a method on its first
//! argument (`xs.map(f)` == `map(xs, f)`) and with the pipe (`xs |> map(f)`).

use std::collections::BTreeMap;

use crate::interp::Interp;
use crate::value::*;

type R = Result<Value, String>;

fn arg(a: &[Value], i: usize) -> Value {
    a.get(i).cloned().unwrap_or(Value::Null)
}
fn num(a: &[Value], i: usize, f: &str) -> Result<f64, String> {
    arg(a, i).as_f64().ok_or_else(|| format!("{f}: argument {} must be a number, got {}", i + 1, arg(a, i).type_name()))
}
fn int(a: &[Value], i: usize, f: &str) -> Result<i64, String> {
    arg(a, i).as_i64().ok_or_else(|| format!("{f}: argument {} must be a whole number, got {}", i + 1, arg(a, i).type_name()))
}
fn list(a: &[Value], i: usize, f: &str) -> Result<Vec<Value>, String> {
    match arg(a, i) {
        Value::List(l) => Ok(l.borrow().clone()),
        Value::Range(x, y) => Ok((x..y).map(Value::Int).collect()),
        other => Err(format!("{f}: argument {} must be a list, got {}", i + 1, other.type_name())),
    }
}
fn text(a: &[Value], i: usize) -> String {
    arg(a, i).to_string()
}
/// Keep ints as ints when the math allows.
fn numeric(x: f64, like: &Value) -> Value {
    if matches!(like, Value::Int(_)) && x.fract() == 0.0 && x.abs() < 9e15 {
        Value::Int(x as i64)
    } else {
        Value::Float(x)
    }
}

pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> u32 {
    let c = |x: f64| x.round().clamp(0.0, 255.0) as u32;
    (c(r) << 24) | (c(g) << 16) | (c(b) << 8) | c(a)
}

fn hsl(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let h = h.rem_euclid(360.0) / 360.0;
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    if s == 0.0 {
        return (l * 255.0, l * 255.0, l * 255.0);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let t = |mut t: f64| {
        t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (t(h + 1.0 / 3.0) * 255.0, t(h) * 255.0, t(h - 1.0 / 3.0) * 255.0)
}

/// The function behind `name`, if built in.
pub fn builtin(name: &str) -> Option<Value> {
    let f = |n: &str, f: fn(&mut Interp, Vec<Value>) -> R| Some(Value::native(n, f));
    match name {
        // ── Output (Z# names and lower-case) ──
        "Printl" | "println" => f(name, |it, a| {
            let s: Vec<String> = a.iter().map(|v| v.to_string()).collect();
            it.output.push(format!("{}\n", s.join(" ")));
            Ok(Value::Null)
        }),
        "Print" | "print" => f(name, |it, a| {
            let s: Vec<String> = a.iter().map(|v| v.to_string()).collect();
            it.output.push(s.join(" "));
            Ok(Value::Null)
        }),
        // ── Conversion / inspection ──
        "len" => f(name, |_, a| {
            Ok(Value::Int(match arg(&a, 0) {
                Value::List(l) => l.borrow().len(),
                Value::Map(m) => m.borrow().len(),
                Value::Str(s) => s.chars().count(),
                Value::Range(x, y) => (y - x).max(0) as usize,
                v => return Err(format!("len: {} has no length", v.type_name())),
            } as i64))
        }),
        "str" | "to_string" => f(name, |_, a| Ok(Value::str(&text(&a, 0)))),
        "int" | "to_int" => f(name, |_, a| match arg(&a, 0) {
            Value::Str(s) => s.trim().parse::<i64>().map(Value::Int).or_else(|_| s.trim().parse::<f64>().map(|x| Value::Int(x as i64))).map_err(|_| format!("int: \"{s}\" isn't a number")),
            v => v.as_i64().map(Value::Int).ok_or(format!("int: can't convert {}", v.type_name())),
        }),
        "float" | "to_float" => f(name, |_, a| match arg(&a, 0) {
            Value::Str(s) => s.trim().parse::<f64>().map(Value::Float).map_err(|_| format!("float: \"{s}\" isn't a number")),
            v => v.as_f64().map(Value::Float).ok_or(format!("float: can't convert {}", v.type_name())),
        }),
        "bool" => f(name, |_, a| Ok(Value::Bool(arg(&a, 0).truthy()))),
        "type" => f(name, |_, a| Ok(Value::str(&arg(&a, 0).type_name()))),
        "range" => f(name, |_, a| {
            if a.len() == 1 {
                Ok(Value::Range(0, int(&a, 0, "range")?))
            } else {
                Ok(Value::Range(int(&a, 0, "range")?, int(&a, 1, "range")?))
            }
        }),
        // ── Math ──
        "abs" => f(name, |_, a| Ok(numeric(num(&a, 0, "abs")?.abs(), &arg(&a, 0)))),
        "floor" => f(name, |_, a| Ok(Value::Int(num(&a, 0, "floor")?.floor() as i64))),
        "ceil" => f(name, |_, a| Ok(Value::Int(num(&a, 0, "ceil")?.ceil() as i64))),
        "round" => f(name, |_, a| Ok(Value::Int(num(&a, 0, "round")?.round() as i64))),
        "sqrt" => f(name, |_, a| Ok(Value::Float(num(&a, 0, "sqrt")?.sqrt()))),
        "sin" => f(name, |_, a| Ok(Value::Float(num(&a, 0, "sin")?.sin()))),
        "cos" => f(name, |_, a| Ok(Value::Float(num(&a, 0, "cos")?.cos()))),
        "tan" => f(name, |_, a| Ok(Value::Float(num(&a, 0, "tan")?.tan()))),
        "atan2" => f(name, |_, a| Ok(Value::Float(num(&a, 0, "atan2")?.atan2(num(&a, 1, "atan2")?)))),
        "pow" => f(name, |_, a| Ok(Value::Float(num(&a, 0, "pow")?.powf(num(&a, 1, "pow")?)))),
        "min" => f(name, |_, a| {
            let xs = if a.len() == 1 { list(&a, 0, "min")? } else { a.clone() };
            xs.into_iter().try_fold(None::<Value>, |m, v| {
                let x = v.as_f64().ok_or("min: needs numbers")?;
                Ok::<_, String>(Some(match m {
                    Some(m) if m.as_f64().unwrap() <= x => m,
                    _ => v,
                }))
            })
            .map(|m| m.unwrap_or(Value::Null))
        }),
        "max" => f(name, |_, a| {
            let xs = if a.len() == 1 { list(&a, 0, "max")? } else { a.clone() };
            xs.into_iter().try_fold(None::<Value>, |m, v| {
                let x = v.as_f64().ok_or("max: needs numbers")?;
                Ok::<_, String>(Some(match m {
                    Some(m) if m.as_f64().unwrap() >= x => m,
                    _ => v,
                }))
            })
            .map(|m| m.unwrap_or(Value::Null))
        }),
        "clamp" => f(name, |_, a| Ok(numeric(num(&a, 0, "clamp")?.clamp(num(&a, 1, "clamp")?, num(&a, 2, "clamp")?), &arg(&a, 0)))),
        "lerp" => f(name, |_, a| {
            let (x, y, t) = (num(&a, 0, "lerp")?, num(&a, 1, "lerp")?, num(&a, 2, "lerp")?);
            Ok(Value::Float(x + (y - x) * t))
        }),
        "random" => f(name, |it, a| {
            let r = it.random();
            Ok(match a.len() {
                0 => Value::Float(r),
                _ => {
                    let (lo, hi) = (int(&a, 0, "random")?, int(&a, 1, "random")?);
                    Value::Int(lo + (r * ((hi - lo + 1).max(1)) as f64) as i64)
                }
            })
        }),
        "PI" => Some(Value::Float(std::f64::consts::PI)),
        "TAU" => Some(Value::Float(std::f64::consts::TAU)),
        // ── Colours ──
        "rgb" => f(name, |_, a| Ok(Value::Color(rgba(num(&a, 0, "rgb")?, num(&a, 1, "rgb")?, num(&a, 2, "rgb")?, 255.0)))),
        "rgba" => f(name, |_, a| {
            let al = num(&a, 3, "rgba")?;
            let al = if al <= 1.0 && !matches!(arg(&a, 3), Value::Int(_)) { al * 255.0 } else { al };
            Ok(Value::Color(rgba(num(&a, 0, "rgba")?, num(&a, 1, "rgba")?, num(&a, 2, "rgba")?, al)))
        }),
        "hsl" => f(name, |_, a| {
            let (r, g, b) = hsl(num(&a, 0, "hsl")?, num(&a, 1, "hsl")?, num(&a, 2, "hsl")?);
            Ok(Value::Color(rgba(r, g, b, 255.0)))
        }),
        "mix" => f(name, |_, a| match (arg(&a, 0), arg(&a, 1)) {
            (Value::Color(x), Value::Color(y)) => {
                let t = num(&a, 2, "mix")?.clamp(0.0, 1.0);
                let (p, q) = (color_parts(x), color_parts(y));
                let m = |u: u8, v: u8| u as f64 + (v as f64 - u as f64) * t;
                Ok(Value::Color(rgba(m(p.0, q.0), m(p.1, q.1), m(p.2, q.2), m(p.3, q.3))))
            }
            _ => {
                let (x, y, t) = (num(&a, 0, "mix")?, num(&a, 1, "mix")?, num(&a, 2, "mix")?);
                Ok(Value::Float(x + (y - x) * t))
            }
        }),
        "with_alpha" => f(name, |_, a| match arg(&a, 0) {
            Value::Color(c) => Ok(Value::Color((c & 0xffff_ff00) | (num(&a, 1, "with_alpha")?.clamp(0.0, 1.0) * 255.0).round() as u32)),
            v => Err(format!("with_alpha: needs a color, got {}", v.type_name())),
        }),
        // ── Lists ──
        "push" => f(name, |it, a| match arg(&a, 0) {
            Value::List(l) => {
                it.check_len(l.borrow().len() + 1)?;
                l.borrow_mut().push(arg(&a, 1));
                Ok(Value::List(l))
            }
            v => Err(format!("push: needs a list, got {}", v.type_name())),
        }),
        "pop" => f(name, |_, a| match arg(&a, 0) {
            Value::List(l) => Ok(l.borrow_mut().pop().unwrap_or(Value::Null)),
            v => Err(format!("pop: needs a list, got {}", v.type_name())),
        }),
        "map" => f(name, |it, a| {
            let xs = list(&a, 0, "map")?;
            let g = arg(&a, 1);
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(it.call_value(&g, vec![x], 0).map_err(|c| it.describe(c))?);
            }
            Ok(Value::list(out))
        }),
        "filter" => f(name, |it, a| {
            let xs = list(&a, 0, "filter")?;
            let g = arg(&a, 1);
            let mut out = vec![];
            for x in xs {
                if it.call_value(&g, vec![x.clone()], 0).map_err(|c| it.describe(c))?.truthy() {
                    out.push(x);
                }
            }
            Ok(Value::list(out))
        }),
        "reduce" => f(name, |it, a| {
            let xs = list(&a, 0, "reduce")?;
            let g = arg(&a, 1);
            let mut acc = arg(&a, 2);
            for x in xs {
                acc = it.call_value(&g, vec![acc, x], 0).map_err(|c| it.describe(c))?;
            }
            Ok(acc)
        }),
        "sum" => f(name, |_, a| {
            let xs = list(&a, 0, "sum")?;
            let all_int = xs.iter().all(|x| matches!(x, Value::Int(_)));
            let s: f64 = xs.iter().map(|x| x.as_f64().unwrap_or(0.0)).sum();
            Ok(if all_int { Value::Int(s as i64) } else { Value::Float(s) })
        }),
        "sort" => f(name, |it, a| {
            let mut xs = list(&a, 0, "sort")?;
            let key = arg(&a, 1);
            let mut keyed = vec![];
            for x in xs.drain(..) {
                let k = if matches!(key, Value::Null) { x.clone() } else { it.call_value(&key, vec![x.clone()], 0).map_err(|c| it.describe(c))? };
                keyed.push((k, x));
            }
            keyed.sort_by(|(p, _), (q, _)| match (p, q) {
                (Value::Str(a), Value::Str(b)) => a.cmp(b),
                _ => p.as_f64().unwrap_or(0.0).partial_cmp(&q.as_f64().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal),
            });
            Ok(Value::list(keyed.into_iter().map(|(_, x)| x).collect()))
        }),
        "reverse" => f(name, |_, a| match arg(&a, 0) {
            Value::Str(s) => Ok(Value::str(&s.chars().rev().collect::<String>())),
            _ => {
                let mut xs = list(&a, 0, "reverse")?;
                xs.reverse();
                Ok(Value::list(xs))
            }
        }),
        "join" => f(name, |_, a| {
            let xs = list(&a, 0, "join")?;
            let sep = if a.len() > 1 { text(&a, 1) } else { String::new() };
            Ok(Value::str(&xs.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(&sep)))
        }),
        "first" => f(name, |_, a| Ok(list(&a, 0, "first")?.into_iter().next().unwrap_or(Value::Null))),
        "last" => f(name, |_, a| Ok(list(&a, 0, "last")?.pop().unwrap_or(Value::Null))),
        "slice" => f(name, |_, a| {
            let xs = list(&a, 0, "slice")?;
            let n = xs.len() as i64;
            let s = int(&a, 1, "slice")?.clamp(0, n) as usize;
            let e = if a.len() > 2 { int(&a, 2, "slice")?.clamp(0, n) as usize } else { n as usize };
            Ok(Value::list(xs[s..e.max(s)].to_vec()))
        }),
        "zip" => f(name, |_, a| {
            let (x, y) = (list(&a, 0, "zip")?, list(&a, 1, "zip")?);
            Ok(Value::list(x.into_iter().zip(y).map(|(p, q)| Value::list(vec![p, q])).collect()))
        }),
        // ── Strings ──
        "upper" => f(name, |_, a| Ok(Value::str(&text(&a, 0).to_uppercase()))),
        "lower" => f(name, |_, a| Ok(Value::str(&text(&a, 0).to_lowercase()))),
        "trim" => f(name, |_, a| Ok(Value::str(text(&a, 0).trim()))),
        "split" => f(name, |_, a| {
            let s = text(&a, 0);
            let sep = if a.len() > 1 { text(&a, 1) } else { " ".into() };
            Ok(Value::list(s.split(sep.as_str()).map(Value::str).collect()))
        }),
        "contains" => f(name, |_, a| match arg(&a, 0) {
            Value::List(l) => Ok(Value::Bool(l.borrow().iter().any(|x| x.equals(&arg(&a, 1))))),
            Value::Map(m) => Ok(Value::Bool(m.borrow().contains_key(&text(&a, 1)))),
            v => Ok(Value::Bool(v.to_string().contains(&text(&a, 1)))),
        }),
        "starts_with" => f(name, |_, a| Ok(Value::Bool(text(&a, 0).starts_with(&text(&a, 1))))),
        "ends_with" => f(name, |_, a| Ok(Value::Bool(text(&a, 0).ends_with(&text(&a, 1))))),
        "replace" => f(name, |_, a| Ok(Value::str(&text(&a, 0).replace(&text(&a, 1), &text(&a, 2))))),
        "chars" => f(name, |_, a| Ok(Value::list(text(&a, 0).chars().map(|c| Value::str(&c.to_string())).collect()))),
        "index_of" => f(name, |_, a| match arg(&a, 0) {
            Value::List(l) => Ok(Value::Int(l.borrow().iter().position(|x| x.equals(&arg(&a, 1))).map(|i| i as i64).unwrap_or(-1))),
            v => Ok(Value::Int(v.to_string().find(&text(&a, 1)).map(|i| i as i64).unwrap_or(-1))),
        }),
        "repeat" => f(name, |it, a| {
            let (s, n) = (text(&a, 0), int(&a, 1, "repeat")?.max(0) as usize);
            it.check_len(s.len().saturating_mul(n))?;
            Ok(Value::str(&s.repeat(n)))
        }),
        // ── Maps ──
        "keys" => f(name, |_, a| match arg(&a, 0) {
            Value::Map(m) => Ok(Value::list(m.borrow().keys().map(|k| Value::str(k)).collect())),
            Value::Struct(s) => Ok(Value::list(s.borrow().fields.iter().map(|(k, _)| Value::str(k)).collect())),
            v => Err(format!("keys: needs a map, got {}", v.type_name())),
        }),
        "values" => f(name, |_, a| match arg(&a, 0) {
            Value::Map(m) => Ok(Value::list(m.borrow().values().cloned().collect())),
            v => Err(format!("values: needs a map, got {}", v.type_name())),
        }),
        "has" => f(name, |_, a| match arg(&a, 0) {
            Value::Map(m) => Ok(Value::Bool(m.borrow().contains_key(&text(&a, 1)))),
            v => Err(format!("has: needs a map, got {}", v.type_name())),
        }),
        "remove" => f(name, |_, a| match arg(&a, 0) {
            Value::Map(m) => Ok(m.borrow_mut().remove(&text(&a, 1)).unwrap_or(Value::Null)),
            Value::List(l) => {
                let i = int(&a, 1, "remove")?;
                let mut l = l.borrow_mut();
                if i < 0 || i as usize >= l.len() {
                    return Err(format!("remove: index {i} is outside the list"));
                }
                Ok(l.remove(i as usize))
            }
            v => Err(format!("remove: needs a map or list, got {}", v.type_name())),
        }),
        "get" => f(name, |_, a| match arg(&a, 0) {
            Value::Map(m) => Ok(m.borrow().get(&text(&a, 1)).cloned().unwrap_or(arg(&a, 2))),
            v => Err(format!("get: needs a map, got {}", v.type_name())),
        }),
        // ── Z# compatibility ──
        "NVec2" | "vec2" => f(name, |it, a| it.construct("Vec2", vec![Value::Float(num(&a, 0, "NVec2")?), Value::Float(num(&a, 1, "NVec2")?)])),
        "assert" => f(name, |_, a| {
            if arg(&a, 0).truthy() {
                Ok(Value::Null)
            } else {
                Err(if a.len() > 1 { format!("assertion failed: {}", text(&a, 1)) } else { "assertion failed".into() })
            }
        }),
        "Canvas" => f(name, |it, a| {
            let (w, h) = (int(&a, 0, "Canvas")?, int(&a, 1, "Canvas")?);
            crate::canvas::Canvas::new_value(it, w, h)
        }),
        _ => None,
    }
}

/// Every built-in global (the docs check that each one is documented).
pub const GLOBALS: &[&str] = &[
    "Printl", "println", "Print", "print", "len", "str", "to_string", "int", "to_int", "float", "to_float", "bool", "type", "range", "abs", "floor",
    "ceil", "round", "sqrt", "sin", "cos", "tan", "atan2", "pow", "min", "max", "clamp", "lerp", "random", "PI", "TAU", "rgb", "rgba", "hsl", "mix",
    "with_alpha", "push", "pop", "map", "filter", "reduce", "sum", "sort", "reverse", "join", "first", "last", "slice", "zip", "upper", "lower",
    "trim", "split", "contains", "starts_with", "ends_with", "replace", "chars", "index_of", "repeat", "keys", "values", "has", "remove", "get",
    "NVec2", "vec2", "assert", "Canvas",
];

pub fn install(it: &mut Interp) {
    for g in GLOBALS {
        if let Some(v) = builtin(g) {
            it.set_global(g, v);
        }
    }
    it.structs.insert("Vec2".into(), vec![("x".into(), None), ("y".into(), None)]);
    // Z#'s namespaces.
    let ns = |pairs: &[(&str, &str)]| {
        let mut m = BTreeMap::new();
        for (zs, ours) in pairs {
            if let Some(v) = builtin(ours) {
                m.insert(zs.to_string(), v);
            }
        }
        m
    };
    let mut zs = BTreeMap::new();
    zs.insert(
        "Math".to_string(),
        Value::map(ns(&[("Sin", "sin"), ("Cos", "cos"), ("Tan", "tan"), ("Sqrt", "sqrt"), ("Abs", "abs"), ("Pow", "pow"), ("Round", "round"), ("Floor", "floor"), ("Ceil", "ceil"), ("Clamp", "clamp"), ("Lerp", "lerp"), ("Random", "random")])),
    );
    let mut sys = BTreeMap::new();
    sys.insert("Command".to_string(), Value::native("ZS.System.Command", |_, _| Err("ZS.System.Command isn't available: Z#++ scripts can't run programs".into())));
    sys.insert("Exit".to_string(), Value::native("ZS.System.Exit", |_, _| Err("exit".into())));
    zs.insert("System".to_string(), Value::map(sys));
    let mut gfx = BTreeMap::new();
    gfx.insert(
        "Init".to_string(),
        Value::native("ZS.Graphics.Init", |it, a| {
            let (w, h) = (int(&a, 1, "ZS.Graphics.Init")?, int(&a, 2, "ZS.Graphics.Init")?);
            let screen = crate::canvas::Canvas::new_value(it, w, h)?;
            it.set_global("screen", screen);
            it.graphics = Some((text(&a, 0), w as u32, h as u32));
            Ok(Value::Null)
        }),
    );
    zs.insert("Graphics".to_string(), Value::map(gfx));
    it.set_global("ZS", Value::map(zs));
}

/// A built-in method on `v`, if there's one called `name`.
pub fn method(_it: &mut Interp, _v: &Value, name: &str) -> Option<Value> {
    if GLOBALS.contains(&name) && !name.starts_with(char::is_uppercase) {
        builtin(name)
    } else {
        None
    }
}
