//! Find in page. The page's matching text is wrapped in `<mark>` elements
//! by a small script run in the page, so it works on any engine that runs
//! JavaScript; the window shows "3/12" and steps through the matches.

/// The script that finds `query` and moves to the next (`step` 1) or
/// previous (`step` -1) match. An empty query removes the highlights.
/// The page answers `"current/total"`.
pub fn script(query: &str, step: i32) -> String {
    let q = serde_json::to_string(query).unwrap_or_else(|_| "\"\"".into());
    format!("({FIND_JS})({q}, {step})")
}

/// "3/12" → (3, 12).
pub fn parse(answer: &str) -> Option<(usize, usize)> {
    let (a, b) = answer.split_once('/')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

const FIND_JS: &str = r##"function (q, step) {
  var S = window.__stonedgemFind || (window.__stonedgemFind = { q: "", marks: [], i: -1 });
  function clear() {
    S.marks.forEach(function (m) {
      var p = m.parentNode;
      if (!p) return;
      p.replaceChild(document.createTextNode(m.textContent), m);
      p.normalize();
    });
    S.marks = []; S.i = -1;
  }
  if (q !== S.q) {
    clear();
    S.q = q;
    if (q && document.body) {
      var ql = q.toLowerCase(), nodes = [], n;
      var w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, { acceptNode: function (t) {
        var p = t.parentNode && t.parentNode.nodeName;
        return p === "SCRIPT" || p === "STYLE" || p === "NOSCRIPT" || p === "TEXTAREA" ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT;
      } });
      while ((n = w.nextNode())) nodes.push(n);
      nodes.forEach(function (t) {
        var s = t.nodeValue, l = s.toLowerCase(), i = l.indexOf(ql);
        if (i < 0) return;
        var f = document.createDocumentFragment(), last = 0;
        while (i >= 0) {
          f.appendChild(document.createTextNode(s.slice(last, i)));
          var m = document.createElement("mark");
          m.style.cssText = "background:#ffd54f;color:#000;border-radius:2px";
          m.textContent = s.slice(i, i + q.length);
          f.appendChild(m); S.marks.push(m);
          last = i + q.length; i = l.indexOf(ql, last);
        }
        f.appendChild(document.createTextNode(s.slice(last)));
        t.parentNode.replaceChild(f, t);
      });
    }
    step = step < 0 ? 0 : 1;
    if (step === 0) S.i = 0;
  }
  var n2 = S.marks.length;
  if (!n2) return "0/0";
  if (S.i >= 0 && S.marks[S.i]) S.marks[S.i].style.background = "#ffd54f";
  S.i = S.i < 0 ? 0 : (S.i + step + n2) % n2;
  var c = S.marks[S.i];
  c.style.background = "#ff9800";
  c.scrollIntoView({ block: "center" });
  return (S.i + 1) + "/" + n2;
}"##;

#[cfg(test)]
mod tests {
    #[test]
    fn script_quotes_the_query() {
        let s = super::script("a\"b</script>", -1);
        assert!(s.ends_with(r#"("a\"b</script>", -1)"#), "{s}");
        assert_eq!(super::parse("3/12"), Some((3, 12)));
        assert_eq!(super::parse("0/0"), Some((0, 0)));
        assert_eq!(super::parse("x"), None);
    }
}
