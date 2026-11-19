# The sandbox

Z#++ runs code from extensions inside a browser, so a script can only do what its host lets it, and every run is limited.

## What a script can reach

Only what the host registers: its own functions, the [built-ins](reference.md), and whatever objects and functions the host adds (a canvas, a page, a store). In particular:

- **No files.** `import` asks the host to read a file, relative to the program's folder, and can't go above it. Absolute paths, `..` past the top, and drive letters are refused.
- **No processes.** Z#'s `ZS.System.Command` exists so Z# programs load, but calling it fails.
- **No network.** There's nothing to open a connection with unless the host provides it.

```zsp error
func Main() {
    ZS.System.Command("rm -rf /")
}
```

## Limits

Each run — loading a program, a call into it, an event — starts with a fresh budget:

| Limit | Default | When exceeded |
|---|---|---|
| Steps (statements and expressions evaluated) | 20,000,000 | “the script ran out of steps” |
| Call depth | 64 | “calls nested deeper than 64” |
| Size of a list or string | 10,000,000 items | “too large” |
| Time | 5 seconds | “the script ran longer than 5s” |
| Canvas | 4096 × 4096 | an error when it's made |

```zsp error
func forever() { return forever() }
func Main() { forever() }
```

```zsp error
func Main() {
    let s = "x" * 100_000_000
}
```

Hitting a limit stops the script with that message. `try`/`catch` can't catch it, so a script can't hide a runaway loop from its host.

A host chooses its own limits:

```rust
use std::time::Duration;
use stonedgem_zsharp::Limits;

program.interp.limits = Limits { steps: 1_000_000, time: Duration::from_millis(200), ..Limits::default() };
```

## Randomness

`random()` uses a generator the host can seed (`interp.seed(n)`), so runs can be repeated exactly — useful for tests and for drawings that should come out the same every time.
