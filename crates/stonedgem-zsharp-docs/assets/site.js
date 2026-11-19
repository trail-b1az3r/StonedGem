// Z#++ docs: copy buttons, the theme switch, and the "On this page" highlight.
(function () {
  // Copy code
  document.querySelectorAll('figure.code button.copy').forEach(function (b) {
    b.addEventListener('click', function () {
      var code = b.closest('figure').querySelector('pre code');
      if (!code || !navigator.clipboard) return;
      navigator.clipboard.writeText(code.innerText).then(function () {
        b.textContent = 'Copied';
        setTimeout(function () { b.textContent = 'Copy'; }, 1500);
      });
    });
  });

  // Theme: system → light → dark
  var root = document.documentElement;
  var btn = document.querySelector('button.theme');
  function label() {
    var t = root.dataset.theme;
    btn.textContent = t === 'light' ? 'Light' : t === 'dark' ? 'Dark' : 'Theme: auto';
  }
  if (btn) {
    label();
    btn.addEventListener('click', function () {
      var t = root.dataset.theme;
      var next = !t ? 'light' : t === 'light' ? 'dark' : '';
      if (next) root.dataset.theme = next; else delete root.dataset.theme;
      try { if (next) localStorage.setItem('zs-theme', next); else localStorage.removeItem('zs-theme'); } catch (e) {}
      label();
    });
  }

  // On narrow screens the contents start closed.
  var wrap = document.querySelector('.nav-wrap');
  if (wrap && window.matchMedia('(max-width: 900px)').matches) wrap.removeAttribute('open');

  // Highlight the section being read.
  var links = Array.prototype.slice.call(document.querySelectorAll('.toc a'));
  if (!links.length || !('IntersectionObserver' in window)) return;
  var byId = {};
  links.forEach(function (a) { byId[a.getAttribute('href').slice(1)] = a; });
  var obs = new IntersectionObserver(function (entries) {
    entries.forEach(function (e) {
      if (!e.isIntersecting) return;
      links.forEach(function (a) { a.classList.remove('on'); });
      var a = byId[e.target.id];
      if (a) a.classList.add('on');
    });
  }, { rootMargin: '-64px 0px -70% 0px' });
  Object.keys(byId).forEach(function (id) {
    var h = document.getElementById(id);
    if (h) obs.observe(h);
  });
})();
