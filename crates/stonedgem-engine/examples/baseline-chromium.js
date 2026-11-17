const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const b = await chromium.launch();
  const urls = fs.readFileSync(process.argv[2], 'utf8').split('\n').map(s => s.trim()).filter(s => s && !s.startsWith('#'));
  for (const u of urls) {
    const p = await b.newPage();
    let status = null;
    try {
      const r = await p.goto(u, { waitUntil: 'load', timeout: 30000 });
      status = r && r.status();
      const d = await p.evaluate(() => [document.title, document.body ? document.body.innerText.length : 0, document.links.length, document.images.length]);
      console.log(JSON.stringify({ url: u, status, title: d[0], textChars: d[1], links: d[2], images: d[3] }));
    } catch (e) { console.log(JSON.stringify({ url: u, status, error: String(e).slice(0, 80) })); }
    await p.close();
  }
  await b.close();
})();
