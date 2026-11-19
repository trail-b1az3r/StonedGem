# Z#++ in StonedGem

## In extensions

A StonedGem extension — an OyNIx `.oynix` package, or a Chrome or Firefox extension — can include a `script.zsp`. It's checked when the extension is installed (an extension whose script doesn't load isn't installed) and runs while the extension is on. Besides the [built-ins](reference.md), it can call:

| Function | Does |
|---|---|
| `page_url()`, `page_title()` | the page the event is about |
| `inject_css(css)` | adds a style sheet to that page |
| `run_js(code)` | runs JavaScript in that page |
| `notify(text)` | shows a short message in the browser window |
| `store_get(key)`, `store_set(key, value)` | keeps values between events while the browser runs |
| `panel_set(property, value)` | sets a property of the extension's Slint panel |

and it handles these [events](events.md):

```zsp
// A page finished loading; the filter is a match on its address.
on page_load "*://*.wikipedia.org/*" (url, title) {
    let visits = (store_get("visits") ?? 0) + 1
    store_set("visits", visits)
    inject_css("#content { max-width: 70ch; margin: auto }")
    notify("Reading mode on ${title} (${visits})")
}

// A content script called chrome.runtime.sendMessage(msg).
on message (msg, sender) {
    if msg.kind == "count" {
        return { visits: store_get("visits") ?? 0, from: sender.url }
    }
}

// The extension's panel.slint invoked one of its callbacks.
on panel (name, args) {
    if name == "clicked" {
        panel_set("count", args[0])
    }
}
```

A `message` handler's first non-`null` return value is the reply the content script receives. Errors in a handler show as a notification naming the extension.

## In the browser

- The **start page banner** is drawn by `crates/stonedgem-chrome/assets/start.zsp` with the shader `start.zsg`, once, and cached.
- **stonedgem://zsharp** is a playground: an editor, Run, the output and the canvas.

## In OyNIx

OyNIx's colour themes — its modes (light, dark, OLED, midnight) and styles (glass, classic, neumorphic, graphite) — are a Z#++ program, [`assets/themes.zsp`](https://github.com/trail-b1az3r/OyNIx/blob/main/assets/themes.zsp) in the OyNIx repository. OyNIx calls its `theme(mode, style, base)` function with the accent's Material colour scheme and applies the colours it returns; a test checks text contrast for every combination.
