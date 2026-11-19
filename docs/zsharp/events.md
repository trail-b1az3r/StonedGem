# Events

A script can react to things the host reports. `on name { … }` handles an event; parameters in parentheses receive what the host passes. A string after the name filters by the event's subject, with `*` matching anything.

```zsp
on load {
    Printl("ready")
}

on page_load "*://*.example.com/*" (url, title) {
    Printl("visited ${title}")
}

on message (msg, sender) {
    return { ok: true, echo: msg }
}
```

The host decides which events exist and what they carry. In StonedGem they are:

| Event | Subject (filter) | Parameters | Return value |
|---|---|---|---|
| `page_load` | the page's address | `url`, `title` | ignored |
| `message` | — | the message, `sender` (`{ url }`) | the reply to `chrome.runtime.sendMessage` |
| `panel` | — | the callback's name, its arguments (a list) | ignored |

See [Z#++ in StonedGem](in-stonedgem.md) for what scripts can do from these handlers.

## How a host emits them

A Rust host calls `emit` (which reports how many handlers ran) or `emit_values` (which collects each handler's `return` value):

```rust
// Runs every `on page_load` whose filter matches the address.
let ran = program.interp.emit("page_load", Some(url), vec![Value::str(url), Value::str(title)])?;

// Collects what the handlers returned.
let replies = program.interp.emit_values("message", None, vec![msg, sender])?;
```

Several handlers for the same event all run, in the order they appear. See [Embedding in Rust](embedding.md).
